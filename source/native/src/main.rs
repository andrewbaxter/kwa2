pub mod interface;
pub mod dbutil;
pub mod fsutil;
pub mod subsystems;
pub mod util;

good_ormning::good_module!(pub db);

use {
    crate::{
        fsutil::create_dirs,
        interface::{
            config::{
                FdapConfig,
                MaybeFdap,
                OidcConfig,
                UsersConfig,
            },
            s2s::{
                s2sv1,
                SpaghRecord,
                SpaghRecordV1,
            },
        },
        subsystems::{
            accounts::{
                build_users_state,
                identify_c2s,
                UsersState,
            },
            oidc::{
                self,
                OidcState,
            },
            s2s::S2sState,
        },
    },
    aargvark::{
        traits_impls::AargvarkJson,
        vark,
        Aargvark,
    },
    deadpool_sqlite::Pool,
    flowcontrol::{
        shed,
        ta_return,
    },
    http::{
        header::{
            ACCEPT_ENCODING,
            AUTHORIZATION,
            CACHE_CONTROL,
            CONTENT_ENCODING,
            CONTENT_TYPE,
            ETAG,
            HOST,
            IF_NONE_MATCH,
            ORIGIN,
            VARY,
            X_CONTENT_TYPE_OPTIONS,
        },
        request::Parts,
        status,
        HeaderMap,
        Method,
        Request,
        Response,
        Uri,
    },
    http_body_util::{
        combinators::BoxBody,
        BodyExt,
    },
    htwrap::htserve::{
        responses::{
            body_full,
            response_200_json,
            response_400,
            response_401,
            response_403,
            response_404,
            Body,
        },
        viserr::{
            ResultVisErr,
            VisErr,
        },
    },
    hyper::{
        body::{
            Bytes,
            Incoming,
        },
        server::conn::http1,
        service::service_fn,
    },
    hyper_util::rt::TokioIo,
    loga::{
        ea,
        fatal,
        ErrContext,
        Log,
        ResultContext,
    },
    rust_embed_for_web::{
        EmbedableFile,
        RustEmbed,
    },
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::{
        wire::c2s,
    },
    spaghettinuum_native::{
        interface::{
            config::shared::{
                GlobalAddrConfig,
                StrSocketAddr,
            },
            wire::api::publish::latest::InfoResponse,
        },
        utils::system_addr::resolve_global_ip,
    },
    std::{
        net::SocketAddr,
        path::PathBuf,
        str::FromStr,
        sync::Arc,
    },
    taskmanager::TaskManager,
    tokio::{
        net::TcpListener,
        runtime,
    },
    tokio_stream::wrappers::TcpListenerStream,
};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Config {
    pub public_url: String,
    pub bind_sockaddr: StrSocketAddr,
    pub spagh_node_bind_sockaddr: StrSocketAddr,
    pub spagh_publisher_bind_sockaddr: StrSocketAddr,
    pub spagh_publisher_advertise_port: Option<u16>,
    pub s2s_bind_sockaddr: StrSocketAddr,
    pub s2s_advertise_port: Option<u16>,
    pub global_addrs: Vec<GlobalAddrConfig>,
    pub cache_dir: PathBuf,
    pub persistent_dir: PathBuf,
    pub oidc_config: OidcConfig,
    pub users: MaybeFdap<UsersConfig>,
    #[serde(default)]
    pub fdap: Option<FdapConfig>,
    #[serde(default = "default_page_size_default")]
    pub default_page_size: usize,
}

fn default_page_size_default() -> usize {
    return 100;
}

#[derive(Aargvark)]
struct Args {
    config: AargvarkJson<Config>,
    #[vark(flag = "--validate")]
    validate: Option<()>,
}

pub struct State {
    log: loga::Log,
    db: Pool,
    oidc_state: OidcState,
    users_state: UsersState,
    s2s_state: S2sState,
    s2s_client: Arc<crate::subsystems::s2s_client::S2sClient>,
    spagh_publish: crate::subsystems::spagh_publish::SpaghPublishState,
    ws_bus: Arc<crate::subsystems::ws_bus::WsBus>,
    vapid_keys: crate::subsystems::push::VapidKeypair,
    default_page_size: usize,
    pull_scheduler: Arc<crate::subsystems::pull_scheduler::PullScheduler>,
    files_dir: PathBuf,
}

fn api_resp<T: Serialize>(state: &State, r: Result<T, loga::Error>) -> Response<Body> {
    match r {
        Ok(v) => {
            return response_200_json(&v);
        },
        Err(e) => {
            let text = e.to_string();
            state.log.log_err(loga::DEBUG, e.context("Error handling api request"));
            return Response::builder()
                .status(status::StatusCode::INTERNAL_SERVER_ERROR)
                .header(CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(body_full(text.into_bytes()))
                .unwrap();
        },
    }
}

fn s2s_error_response(state: &State, e: crate::subsystems::s2s_error::S2sError) -> Response<Body> {
    if let crate::subsystems::s2s_error::S2sError::Internal(inner) = &e {
        state.log.log_err(loga::WARN, inner.clone().context("Error handling s2s request"));
    }
    return Response::builder()
        .status(e.status())
        .header(CONTENT_TYPE, "application/json")
        .header(CACHE_CONTROL, crate::subsystems::s2s_error::CachePolicy::NoStore.header_value())
        .body(body_full(serde_json::to_vec(&e.body()).unwrap()))
        .unwrap();
}

fn s2s_respond<
    R: s2sv1::ReqTrait,
>(state: &State, r: Result<R::Resp, crate::subsystems::s2s_error::S2sError>) -> Response<Body> {
    match r {
        Ok(v) => return Response::builder()
            .status(200)
            .header(CONTENT_TYPE, "application/json")
            .header(CACHE_CONTROL, crate::subsystems::s2s_error::CachePolicy::NoStore.header_value())
            .body(body_full(serde_json::to_vec(&v).unwrap()))
            .unwrap(),
        Err(e) => return s2s_error_response(state, e),
    }
}

fn s2s_respond_get<
    R: c2s::PathReqTrait,
>(
    state: &State,
    r: Result<crate::subsystems::s2s_error::Cached<R::Resp>, crate::subsystems::s2s_error::S2sError>,
) -> Response<Body>
where
    R::Resp: Serialize {
    match r {
        Ok(v) => return Response::builder()
            .status(200)
            .header(CONTENT_TYPE, "application/json")
            .header(CACHE_CONTROL, v.policy.header_value())
            .body(body_full(serde_json::to_vec(&v.value).unwrap()))
            .unwrap(),
        Err(e) => return s2s_error_response(state, e),
    }
}

fn auth_token(head: &Parts) -> String {
    return head
        .headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .unwrap_or_default();
}

fn accepts_encoding(headers: &HeaderMap, encoding: &str) -> bool {
    for v in headers.get_all(ACCEPT_ENCODING) {
        let Ok(v) = v.to_str() else {
            continue;
        };
        for item in v.split(',') {
            let mut parts = item.split(';');
            if !parts.next().unwrap().trim().eq_ignore_ascii_case(encoding) {
                continue;
            }
            let mut q = 1.;
            for param in parts {
                let Some((k, v)) = param.split_once('=') else {
                    continue;
                };
                if !k.trim().eq_ignore_ascii_case("q") {
                    continue;
                }
                q = v.trim().parse::<f32>().unwrap_or(0.);
            }
            return q > 0.;
        }
    }
    return false;
}

fn vis_response(
    state: &Arc<State>,
    url: Uri,
    res: Result<Response<Body>, VisErr<loga::Error>>,
) -> Response<BoxBody<Bytes, std::io::Error>> {
    match res {
        Ok(r) => {
            return r;
        },
        Err(e) => {
            match e {
                VisErr::External(e) => {
                    return Response::builder()
                        .status(status::StatusCode::BAD_REQUEST)
                        .body(
                            http_body_util::Full::new(Bytes::from(e.into_bytes()))
                                .map_err(|_| std::io::Error::other(""))
                                .boxed(),
                        )
                        .unwrap();
                },
                VisErr::Internal(e) => {
                    state.log.log_err(loga::WARN, e.context_with("Error serving response", ea!(url = url)));
                    return Response::builder()
                        .status(503)
                        .body(
                            http_body_util::Full::new(Bytes::new()).map_err(|_| std::io::Error::other("")).boxed(),
                        )
                        .unwrap();
                },
            }
        },
    }
}

fn main() {
    let log = Log::new_root(loga::DEBUG);
    let runtime = runtime::Builder::new_current_thread().enable_all().build().unwrap();
    match runtime.block_on({
        let log = log.clone();
        async move {
            ta_return!((), loga::Error);
            let tm = TaskManager::new();
            let args = vark::<Args>();
            let config = args.config.value;
            if args.validate.is_some() {
                eprintln!("Config OK");
                return Ok(());
            }
            create_dirs(&config.persistent_dir).await?;
            let db_path = config.persistent_dir.join("db.sqlite3");
            let files_dir = config.persistent_dir.join("files");
            create_dirs(&files_dir).await?;

            // Spagh
            let mut global_ips = vec![];
            for addr in &config.global_addrs {
                global_ips.push(resolve_global_ip(&log, addr).await?);
            }
            let first_global_ip =
                *global_ips.first().context("Config `global_addrs` must contain at least one address")?;
            let spagh_node_bind_port =
                config.spagh_node_bind_sockaddr.resolve().context("Error resolving spagh node bind addr")?.port();
            let spagh_node =
                spaghettinuum_native::service::node::Node::new(
                    &log,
                    &tm,
                    config.spagh_node_bind_sockaddr,
                    global_ips
                        .iter()
                        .filter(|ip| ip.is_ipv6())
                        .chain(global_ips.iter().filter(|ip| ip.is_ipv4()))
                        .map(|ip| SocketAddr::new(*ip, spagh_node_bind_port))
                        .collect(),
                    &spaghettinuum_native::service::node::default_bootstrap(),
                    &config.cache_dir,
                ).await?;
            let spagh_publisher_bind_sockaddr = config.spagh_publisher_bind_sockaddr.resolve()?;
            let spagh_publisher_advertise_addr =
                SocketAddr::new(
                    first_global_ip,
                    config.spagh_publisher_advertise_port.unwrap_or(spagh_publisher_bind_sockaddr.port()),
                );
            let spagh_publisher =
                spaghettinuum_native::service::publisher::Publisher::new(
                    &log,
                    &tm,
                    spagh_node.clone(),
                    spagh_publisher_bind_sockaddr,
                    spagh_publisher_advertise_addr,
                    &config.persistent_dir,
                ).await?;
            let spagh_resolver =
                spaghettinuum_native::service::resolver::Resolver::new(
                    &log,
                    &tm,
                    spagh_node.clone(),
                    None,
                    &config.cache_dir,
                    Some((spagh_publisher_advertise_addr, spagh_publisher.clone())),
                ).await?;

            // Db
            let db =
                deadpool_sqlite::Config::new(&db_path)
                    .builder(deadpool_sqlite::Runtime::Tokio1)
                    .context("Error creating sqlite pool builder")?
                    .build()
                    .context("Error creating sqlite pool")?;
            db.get().await?.interact(move |conn| -> Result<_, loga::Error> {
                db::migrate(&mut *conn, Some(&|v| {
                    let db::DbVersions::V2(db) = v else {
                        return Ok(());
                    };
                    for (
                        table,
                        column,
                    ) in [
                        ("account", "soft_deleted_at"),
                        ("identity", "soft_deleted_at"),
                        ("channel", "deleted"),
                        ("identity_invite", "expiry"),
                        ("channel_invite", "expiry"),
                        ("message", "receive_time"),
                        ("outbox", "created"),
                        ("session", "last_used"),
                    ] {
                        let query = format!("update \"{}\" set \"{}\" = \"{}\" * 1000", table, column, column);
                        good_ormning::runtime::sqlite::SqliteConnection::execute(
                            &mut db.0,
                            &query,
                            (),
                        ).map_err(
                            |e| good_ormning::runtime::GoodError(format!("Error executing query [{}]: {}", query, e)),
                        )?;
                    }
                    return Ok(());
                })).map_err(|e| loga::err(e.0))?;
                return Ok(());
            }).await?.context_with("Migration failed", ea!(action = "db_init", path = db_path.to_string_lossy()))?;

            // State
            let s2s_bind_sockaddr = config.s2s_bind_sockaddr.resolve().context("Error resolving s2s bind addr")?;
            let vapid_keys = crate::subsystems::push::ensure_vapid_keypair(&db).await?;
            let users_state = build_users_state(&config.users, &config.fdap)?;
            let state = Arc::new(State {
                log: log.clone(),
                db: db,
                oidc_state: oidc::new_state(&log, &config.public_url, config.oidc_config).await?,
                users_state: users_state,
                s2s_state: S2sState::new(spagh_resolver.clone()),
                s2s_client: Arc::new(
                    crate::subsystems::s2s_client::S2sClient::new(log.clone(), spagh_resolver.clone()),
                ),
                spagh_publish: crate::subsystems::spagh_publish::SpaghPublishState::new(
                    spagh_publisher.clone(),
                    InfoResponse {
                        advertise_addr: spagh_publisher_advertise_addr,
                        cert_pub_hash: spagh_publisher.pub_cert_hash(),
                    },
                    SpaghRecord::V1(SpaghRecordV1 {
                        ips: global_ips,
                        port: config.s2s_advertise_port.unwrap_or(s2s_bind_sockaddr.port()),
                    }),
                ),
                ws_bus: Arc::new(crate::subsystems::ws_bus::WsBus::new()),
                vapid_keys: vapid_keys,
                default_page_size: config.default_page_size,
                pull_scheduler: crate::subsystems::pull_scheduler::PullScheduler::new(),
                files_dir: files_dir,
            });
            crate::subsystems::spagh_publish::publish_all(&state).await?;
            tm.periodic("spagh_cert_refresh", crate::subsystems::spagh_publish::CERT_REFRESH_PERIOD, {
                let state = state.clone();
                move || {
                    let state = state.clone();
                    return async move {
                        match crate::subsystems::spagh_publish::refresh_certs(&state).await {
                            Ok(_) => { },
                            Err(e) => {
                                state.log.log_err(loga::WARN, e.context("Error refreshing identity TLS certs"));
                            },
                        }
                    };
                }
            });
            crate::subsystems::pull_scheduler::enumerate_and_seed(&state).await?;
            tm.critical_task("pull_scheduler", {
                let scheduler = state.pull_scheduler.clone();
                let state = state.clone();
                async move {
                    return scheduler.run(state).await;
                }
            });
            tm.critical_task("backfill", {
                let state = state.clone();
                async move {
                    return crate::subsystems::messages::run_backfill(state).await;
                }
            });
            tm.critical_task("retention_cleanup", {
                let state = state.clone();
                async move {
                    return crate::subsystems::messages::run_retention_cleanup(state).await;
                }
            });

            // Serve
            let s2s_tls = tokio_rustls::TlsAcceptor::from(Arc::new({
                let mut server_config =
                    spaghettinuum_native
                    ::utils
                    ::tls_util
                    ::server_config_builder().with_cert_resolver(state.spagh_publish.certs.clone());
                server_config.alpn_protocols = vec![b"http/1.1".to_vec()];
                server_config
            }));
            tm.critical_stream(
                "s2s",
                TcpListenerStream::new(
                    TcpListener::bind(s2s_bind_sockaddr)
                        .await
                        .context_with("Error binding to s2s address", ea!(addr = s2s_bind_sockaddr))?,
                ),
                {
                    let state = state.clone();
                    move |conn| {
                        let state = state.clone();
                        let s2s_tls = s2s_tls.clone();
                        return async move {
                            let conn = match conn {
                                Ok(c) => c,
                                Err(e) => {
                                    state.log.log_err(loga::DEBUG, e.context("Error receiving s2s connection"));
                                    return Ok(());
                                },
                            };
                            tokio::task::spawn(async move {
                                match async {
                                    ta_return!((), loga::Error);
                                    let io =
                                        TokioIo::new(
                                            s2s_tls.accept(conn).await.context("Error during s2s TLS handshake")?,
                                        );
                                    http1::Builder::new().serve_connection(io, service_fn(cap_fn!((req)(state) {
                                        let req: Request<Incoming> = req;
                                        let url = req.uri().clone();
                                        let res = async {
                                            ta_return!(Response < Body >, VisErr < loga:: Error >);
                                            let state = &state;
                                            let (head, body) = req.into_parts();
                                            let path = head.uri.path().to_string();
                                            let Some(rest) =
                                                path.strip_prefix(crate::subsystems::s2s_client::S2SV1_PATH) else {
                                                    return Ok(response_404());
                                                };
                                            let rest = rest.trim_start_matches('/');
                                            if head.method == Method::POST && rest.is_empty() {
                                                return async {
                                                    ta_return!(Response < Body >, VisErr < loga:: Error >);
                                                    let head = &head;

                                                    use crate::{
                                                        interface::s2s::*,
                                                        subsystems::messages as m,
                                                        subsystems::s2s_error::S2sError,
                                                    };

                                                    let auth_token = auth_token(head);
                                                    let body_bytes = body.collect().await.err_external()?.to_bytes();
                                                    let req = match serde_json::from_slice::<s2sv1::Req>(&body_bytes) {
                                                        Ok(r) => r,
                                                        Err(e) => {
                                                            return Ok(
                                                                s2s_error_response(
                                                                    state,
                                                                    S2sError::BadRequest(
                                                                        format!(
                                                                            "Request body is not a valid s2s request: {}",
                                                                            e
                                                                        )
                                                                    ),
                                                                ),
                                                            );
                                                        },
                                                    };
                                                    match req {
                                                        s2sv1::Req::StartIdentify(r) => {
                                                            return Ok(
                                                                s2s_respond::<StartIdentify>(
                                                                    state,
                                                                    Ok(state.s2s_state.start_identify(r).await)
                                                                )
                                                            );
                                                        },
                                                        s2sv1::Req::Identify(r) => {
                                                            return Ok(
                                                                s2s_respond::<Identify>(
                                                                    state,
                                                                    state.s2s_state.identify(&state.db, r).await
                                                                )
                                                            );
                                                        },
                                                        s2sv1::Req::Notify(r) => {
                                                            return Ok(
                                                                s2s_respond::<Notify>(
                                                                    state,
                                                                    m::handle_notify(state, r).await
                                                                )
                                                            );
                                                        },
                                                        s2sv1::Req::Join(r) => {
                                                            return Ok(
                                                                s2s_respond::<Join>(
                                                                    state,
                                                                    m::handle_join(state, &auth_token, r).await
                                                                )
                                                            );
                                                        },
                                                        s2sv1::Req::PullOutbox(r) => {
                                                            return Ok(
                                                                s2s_respond::<PullOutbox>(
                                                                    state,
                                                                    m::handle_pull_outbox(state, &auth_token, r).await
                                                                )
                                                            );
                                                        },
                                                    }
                                                }.await;
                                            }
                                            if head.method == Method::GET && !rest.is_empty() {
                                                return Ok(async {
                                                    let head = &head;

                                                    use crate::{
                                                        interface::s2s::*,
                                                        subsystems::messages as m,
                                                        subsystems::s2s_error::S2sError,
                                                    };

                                                    let auth_token = auth_token(head);
                                                    let first = rest.split('/').next().unwrap();

                                                    macro_rules! dispatch{
                                                        ($req: ty, $handler: expr) => {
                                                            {
                                                                let req = match < $req as c2s:: PathReqTrait >:: deserialize_path(
                                                                    rest
                                                                ) {
                                                                    Ok(r) => r,
                                                                    Err(
                                                                        e
                                                                    ) => return s2s_error_response(
                                                                        state,
                                                                        S2sError::BadRequest(e)
                                                                    ),
                                                                };
                                                                return s2s_respond_get::<$req >(
                                                                    state,
                                                                    $handler(state, &auth_token, req).await
                                                                );
                                                            }
                                                        };
                                                    }

                                                    match first {
                                                        GET_PREFIX_CHANNEL_INFO => dispatch!(
                                                            GetChannelInfo,
                                                            m::handle_channel_info
                                                        ),
                                                        GET_PREFIX_ACTIVITY => dispatch!(
                                                            GetActivity,
                                                            m::handle_get_activity
                                                        ),
                                                        GET_PREFIX_SNAP_PAGE => dispatch!(
                                                            GetSnapPage,
                                                            m::handle_get_snap_page
                                                        ),
                                                        GET_PREFIX_SNAP_PAGE_CONTAINING_TIME => dispatch!(
                                                            GetSnapPageContainingTime,
                                                            m::handle_snap_page_containing_time
                                                        ),
                                                        GET_PREFIX_SNAP_LOOKUP => dispatch!(
                                                            GetSnapLookup,
                                                            m::handle_snap_lookup
                                                        ),
                                                        GET_PREFIX_MESSAGE_HISTORY => dispatch!(
                                                            GetMessageHistory,
                                                            m::handle_get_message_history
                                                        ),
                                                        GET_PREFIX_JOIN_PREVIEW => dispatch!(
                                                            GetJoinPreview,
                                                            m::handle_join_preview
                                                        ),
                                                        GET_PREFIX_FILE => {
                                                            let req =
                                                                match <GetFile as c2s::PathReqTrait>::deserialize_path(
                                                                    rest
                                                                ) {
                                                                    Ok(r) => r,
                                                                    Err(e) => return s2s_error_response(
                                                                        state,
                                                                        S2sError::BadRequest(e)
                                                                    ),
                                                                };
                                                            match m::handle_get_file(state, &auth_token, req).await {
                                                                Ok((mime, bytes)) => {
                                                                    return Response::builder()
                                                                        .status(200)
                                                                        .header(CONTENT_TYPE, mime)
                                                                        .header(X_CONTENT_TYPE_OPTIONS, "nosniff")
                                                                        .header(
                                                                            CACHE_CONTROL,
                                                                            crate
                                                                            ::subsystems
                                                                            ::s2s_error
                                                                            ::CachePolicy
                                                                            ::Immutable.header_value()
                                                                        )
                                                                        .body(body_full(bytes))
                                                                        .unwrap();
                                                                },
                                                                Err(e) => return s2s_error_response(state, e),
                                                            }
                                                        },
                                                        _ => return response_404(),
                                                    }
                                                }.await);
                                            }
                                            return Ok(response_404());
                                        }.await;
                                        return Ok(vis_response(&state, url, res)) as Result<_, std::io::Error>;
                                    }))).await?;
                                    return Ok(());
                                }.await {
                                    Ok(_) => (),
                                    Err(e) => {
                                        state.log.log_err(loga::DEBUG, e.context("Error serving s2s connection"));
                                    },
                                }
                            });
                            return Ok(());
                        };
                    }
                },
            );
            tm.critical_stream(
                "http",
                TcpListenerStream::new(
                    TcpListener::bind(config.bind_sockaddr.resolve().context("Error resolving server bind addr")?)
                        .await
                        .context_with("Error binding to address", ea!(addr = config.bind_sockaddr))?,
                ),
                {
                    move |conn| {
                        let state = state.clone();
                        return async move {
                            let conn = match conn {
                                Ok(c) => c,
                                Err(e) => {
                                    state.log.log_err(loga::DEBUG, e.context("Error receiving request"));
                                    return Ok(());
                                },
                            };
                            let io = TokioIo::new(conn);
                            tokio::task::spawn(async move {
                                match async {
                                    ta_return!((), loga::Error);
                                    http1::Builder::new().serve_connection(io, service_fn(cap_fn!((req)(state) {
                                        let req: Request<Incoming> = req;
                                        let url = req.uri().clone();
                                        let res = async {
                                            ta_return!(Response < Body >, VisErr < loga:: Error >);
                                            let state = &state;
                                            let is_ws = hyper_tungstenite::is_upgrade_request(&req);
                                            let (head, body) = req.into_parts();
                                            let path = head.uri.path().to_string();
                                            let mut segs = path.trim_start_matches('/').split('/');
                                            let first = segs.next().unwrap();
                                            if is_ws {
                                                if !path.trim_start_matches('/').is_empty() {
                                                    return Ok(response_404());
                                                }
                                                let same_origin = shed!{
                                                    let Some(origin) =
                                                        head.headers.get(ORIGIN).and_then(|o| o.to_str().ok()) else {
                                                            break false;
                                                        };
                                                    let Some(host) =
                                                        head.headers.get(HOST).and_then(|h| h.to_str().ok()) else {
                                                            break false;
                                                        };
                                                    let Ok(origin) = Uri::from_str(origin) else {
                                                        break false;
                                                    };
                                                    break origin
                                                        .authority()
                                                        .map(|a| a.as_str() == host)
                                                        .unwrap_or(false);
                                                };
                                                if !same_origin {
                                                    return Ok(response_403());
                                                }
                                                let Some(account) =
                                                    identify_c2s(state, &head.headers).await.err_internal()? else {
                                                        return Ok(response_401());
                                                    };
                                                let (response, ws) =
                                                    hyper_tungstenite::upgrade(Request::from_parts(head, body), None)
                                                        .map_err(|e| loga::err(e.to_string()))
                                                        .err_external()?;
                                                let state = state.clone();
                                                tokio::spawn(async move {
                                                    let ws = match ws.await {
                                                        Ok(w) => w,
                                                        Err(e) => {
                                                            state
                                                                .log
                                                                .log_err(
                                                                    loga::WARN,
                                                                    loga::err(
                                                                        e.to_string()
                                                                    ).context("WS upgrade failed")
                                                                );
                                                            return;
                                                        },
                                                    };
                                                    crate::subsystems::ws_bus::run_ws_connection(
                                                        &state,
                                                        account,
                                                        ws
                                                    ).await;
                                                });
                                                return Ok(
                                                    response.map(
                                                        |b| b.map_err(|e| std::io::Error::other(e.to_string())).boxed()
                                                    )
                                                );
                                            }
                                            match first {
                                                "oidc" => {
                                                    return Ok(
                                                        oidc::handle_oidc(&state.oidc_state, &state.db, head).await?
                                                    );
                                                },
                                                "logout" => {
                                                    return Ok(
                                                        oidc::handle_logout(&state.oidc_state, &state.db, head).await?
                                                    );
                                                },
                                                "api" => {
                                                    if head.method != Method::POST {
                                                        return Ok(response_404());
                                                    }
                                                    let is_json =
                                                        head
                                                            .headers
                                                            .get(CONTENT_TYPE)
                                                            .and_then(|v| v.to_str().ok())
                                                            .map(
                                                                |v| v
                                                                    .split(';')
                                                                    .next()
                                                                    .unwrap()
                                                                    .trim()
                                                                    .eq_ignore_ascii_case("application/json")
                                                            )
                                                            .unwrap_or(false);
                                                    if !is_json {
                                                        return Ok(
                                                            response_400("Content-Type must be application/json")
                                                        );
                                                    }
                                                    let Some(account) =
                                                        identify_c2s(state, &head.headers).await.err_internal()? else {
                                                            return Ok(response_401());
                                                        };
                                                    let req =
                                                        serde_json::from_slice::<c2s::proto::Req>(
                                                            &body.collect().await.err_external()?.to_bytes(),
                                                        ).err_external()?;
                                                    return Ok(async {
                                                        use crate::subsystems::c2s as h;

                                                        match req.to_server_req() {
                                                            c2s::proto::ServerReq::Logout(_, _) => {
                                                                return api_resp(
                                                                    state,
                                                                    oidc::delete_req_session(
                                                                        &state.db,
                                                                        &state.log,
                                                                        &head.headers
                                                                    ).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::NotificationRegister(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::notification_register(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::IdentityCreate(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::identity_create(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::IdentityModify(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::identity_modify(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::IdentityDelete(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::identity_delete(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::IdentityList(_, _) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::identity_list(state, account).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::IdentityInviteCreate(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::identity_invite_create(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::IdentityInviteModify(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::identity_invite_modify(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::IdentityInviteDelete(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::identity_invite_delete(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::IdentityInviteList(_, _) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::identity_invite_list(state, account).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelGroupCreate(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_group_create(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelGroupModify(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_group_modify(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelGroupDelete(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_group_delete(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelGroupList(_, _) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_group_list(state, account).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelCreate(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_create(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelJoinChannel(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_join_channel(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelJoinIdentity(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_join_identity(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelAdd(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_add(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelJoinPreview(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_join_preview(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelModify(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_modify(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelDelete(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_delete(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelList(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_list(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelMemberList(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_member_list(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelMemberModify(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_member_modify(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelMemberDelete(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_member_delete(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelInviteCreate(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_invite_create(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelInviteModify(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_invite_modify(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelInviteDelete(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_invite_delete(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ChannelInviteList(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::channel_invite_list(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ContactList(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::contact_list(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::ContactModify(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::contact_modify(state, account, r).await
                                                                );
                                                            },
                                                            c2s::proto::ServerReq::MessagePush(_, r) => {
                                                                return api_resp(
                                                                    state,
                                                                    h::message_push(state, account, r).await
                                                                );
                                                            },
                                                        }
                                                    }.await);
                                                },
                                                p if crate::subsystems::c2s_get::GET_PATH_PREFIXES.contains(&p) => {
                                                    if head.method != Method::GET {
                                                        return Ok(response_404());
                                                    }
                                                    let Some(account) =
                                                        identify_c2s(state, &head.headers).await.err_internal()? else {
                                                            return Ok(response_401());
                                                        };
                                                    return crate::subsystems::c2s_get::handle(
                                                        state,
                                                        account,
                                                        &path
                                                    ).await;
                                                },
                                                _ => {
                                                    return Ok(async {
                                                        #[derive(RustEmbed)]
                                                        #[folder = "$STATIC_DIR"]
                                                        struct Static;

                                                        let path = head.uri.path().trim_start_matches('/');
                                                        let path = if path.is_empty() {
                                                            "index.html"
                                                        } else {
                                                            path
                                                        };
                                                        let versioned = path.starts_with("v/");
                                                        let Some(f) = Static::get(path) else {
                                                            return response_404();
                                                        };
                                                        let (encoding, data) = shed!{
                                                            'found _;
                                                            if accepts_encoding(&head.headers, "br") {
                                                                if let Some(d) = f.data_br() {
                                                                    break 'found (Some("br"), d);
                                                                }
                                                            }
                                                            if accepts_encoding(&head.headers, "gzip") {
                                                                if let Some(d) = f.data_gzip() {
                                                                    break 'found (Some("gzip"), d);
                                                                }
                                                            }
                                                            break 'found (None, f.data());
                                                        };
                                                        let etag = match encoding {
                                                            Some(e) => format!("\"{}-{}\"", f.hash(), e),
                                                            None => f.etag().to_string(),
                                                        };
                                                        if let Some(h) = head.headers.get(IF_NONE_MATCH) {
                                                            if h == etag.as_bytes() {
                                                                let mut resp =
                                                                    Response::builder()
                                                                        .status(304)
                                                                        .header(ETAG, etag)
                                                                        .header(VARY, ACCEPT_ENCODING.as_str());
                                                                if versioned {
                                                                    resp =
                                                                        resp.header(
                                                                            CACHE_CONTROL,
                                                                            crate
                                                                            ::subsystems
                                                                            ::s2s_error
                                                                            ::CachePolicy
                                                                            ::Immutable.header_value()
                                                                        );
                                                                }
                                                                return resp.body(body_full(vec![])).unwrap();
                                                            }
                                                        }
                                                        let mut resp =
                                                            Response::builder()
                                                                .status(200)
                                                                .header(
                                                                    CONTENT_TYPE,
                                                                    f.mime_type().unwrap_or("application/octet-stream")
                                                                )
                                                                .header(ETAG, etag)
                                                                .header(VARY, ACCEPT_ENCODING.as_str());
                                                        if versioned {
                                                            resp =
                                                                resp.header(
                                                                    CACHE_CONTROL,
                                                                    crate
                                                                    ::subsystems
                                                                    ::s2s_error
                                                                    ::CachePolicy
                                                                    ::Immutable.header_value()
                                                                );
                                                        }
                                                        if let Some(e) = encoding {
                                                            resp = resp.header(CONTENT_ENCODING, e);
                                                        }
                                                        return resp.body(body_full(data.to_vec())).unwrap();
                                                    }.await);
                                                },
                                            }
                                        }.await;
                                        return Ok(vis_response(&state, url, res)) as Result<_, std::io::Error>;
                                    }))).with_upgrades().await?;
                                    return Ok(());
                                }.await {
                                    Ok(_) => (),
                                    Err(e) => {
                                        state.log.log_err(loga::DEBUG, e.context("Error serving connection"));
                                    },
                                }
                            });
                            return Ok(());
                        };
                    }
                },
            );
            tm.join(&log).await?;
            return Ok(());
        }
    }) {
        Ok(_) => { },
        Err(e) => {
            fatal(e);
        },
    }
}
