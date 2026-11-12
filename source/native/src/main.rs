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
                SpaghRecordV1,
            },
        },
        subsystems::{
            accounts::{
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
        interface::config::shared::{
            GlobalAddrConfig,
            StrSocketAddr,
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
    pub ffmpeg: PathBuf,
    #[serde(default = "max_file_size_default")]
    pub max_file_size: u64,
    #[serde(default = "file_cache_size_default")]
    pub file_cache_size: u64,
}

fn default_page_size_default() -> usize {
    return 100;
}

fn max_file_size_default() -> u64 {
    return 1024 * 1024 * 1024;
}

fn file_cache_size_default() -> u64 {
    return 10 * 1024 * 1024 * 1024;
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
    file_cache_dir: PathBuf,
    file_cache_fills: std
    ::sync
    ::Mutex<std::collections::HashSet<(spaghettinuum::interface::identity::Identity, String, bool)>>,
    ffmpeg: PathBuf,
    max_file_size: u64,
    file_cache_size: u64,
    calendar_cache: crate::subsystems::calendar::CalendarCache,
    calendar_page_cache: crate::subsystems::calendar::CalendarPageCache,
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
    use {
        crate::{
            interface::s2s::{
                S2sErrorBody,
                S2sErrorKind,
            },
            subsystems::s2s_error::S2sError,
        },
        http::StatusCode,
    };

    let (status, body) = match &e {
        S2sError::BadRequest(m) => (StatusCode::BAD_REQUEST, S2sErrorBody {
            kind: S2sErrorKind::BadRequest,
            message: m.clone(),
        }),
        S2sError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, S2sErrorBody {
            kind: S2sErrorKind::Unauthorized,
            message: m.clone(),
        }),
        S2sError::Forbidden(m) => (StatusCode::FORBIDDEN, S2sErrorBody {
            kind: S2sErrorKind::Forbidden,
            message: m.clone(),
        }),
        S2sError::NotFound(m) => (StatusCode::NOT_FOUND, S2sErrorBody {
            kind: S2sErrorKind::NotFound,
            message: m.clone(),
        }),
        S2sError::Internal(inner) => {
            state.log.log_err(loga::WARN, inner.clone().context("Error handling s2s request"));
            (StatusCode::INTERNAL_SERVER_ERROR, S2sErrorBody {
                kind: S2sErrorKind::Internal,
                message: "Internal error".to_string(),
            })
        },
    };
    return Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .header(CACHE_CONTROL, crate::subsystems::s2s_error::CachePolicy::NoStore.header_value())
        .body(body_full(serde_json::to_vec(&body).unwrap()))
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
            let file_cache_dir = config.cache_dir.join("files");
            create_dirs(&file_cache_dir).await?;

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
            let vapid_keys = {
                use {
                    crate::{
                        db,
                        dbutil::tx,
                    },
                    base64::{
                        engine::general_purpose::URL_SAFE_NO_PAD,
                        Engine,
                    },
                    loga::{
                        ResultContext,
                    },
                    p256::{
                        elliptic_curve::{
                            sec1::ToSec1Point,
                            Generate,
                        },
                        SecretKey,
                    },
                };
                use crate::subsystems::push::*;

                let ret: Result<VapidKeypair, loga::Error> = async {
                    let (pool,): (&deadpool_sqlite::Pool,) = (&db,);
                    let existing = tx(pool, move |db_tx| {
                        let priv_row = good_ormning::sqlite::good_query_opt!(
                            db,
                            r#"select value from server_settings where key = ${string = KEY_VAPID_PRIVATE_B64}"#;
                            db_tx
                        ).map_err(|e| loga::err(e.0))?;
                        let pub_row = good_ormning::sqlite::good_query_opt!(
                            db,
                            r#"select value from server_settings where key = ${string = KEY_VAPID_PUBLIC_B64}"#;
                            db_tx
                        ).map_err(|e| loga::err(e.0))?;
                        return Ok((priv_row, pub_row));
                    }).await.context("Error reading VAPID keys from server_settings")?;
                    if let (Some(priv_b64), Some(pub_b64)) = existing {
                        return Ok(VapidKeypair {
                            private_b64url: priv_b64,
                            public_b64url: pub_b64,
                        });
                    }
                    let secret = <SecretKey as Generate>::generate_from_rng(&mut rand::rng());
                    let priv_bytes = secret.to_bytes();
                    let pub_point = secret.public_key().to_sec1_point(false);
                    let priv_b64 = URL_SAFE_NO_PAD.encode(priv_bytes.as_slice());
                    let pub_b64 = URL_SAFE_NO_PAD.encode(pub_point.as_bytes());
                    let priv_b64_c = priv_b64.clone();
                    let pub_b64_c = pub_b64.clone();
                    tx(pool, move |db_tx| {
                        good_ormning::sqlite::good_query!(
                            db,
                            r#"insert into server_settings (key, value) values (${string = KEY_VAPID_PRIVATE_B64}, ${string = priv_b64_c.as_str()}) on conflict (key) do nothing"#;
                            db_tx
                        ).map_err(|e| loga::err(e.0))?;
                        good_ormning::sqlite::good_query!(
                            db,
                            r#"insert into server_settings (key, value) values (${string = KEY_VAPID_PUBLIC_B64}, ${string = pub_b64_c.as_str()}) on conflict (key) do nothing"#;
                            db_tx
                        ).map_err(|e| loga::err(e.0))?;
                        return Ok(());
                    }).await.context("Error persisting new VAPID keypair")?;
                    return Ok(VapidKeypair {
                        private_b64url: priv_b64,
                        public_b64url: pub_b64,
                    });
                }.await;
                ret
            }?;
            let users_state = {
                use {
                    crate::interface::config::{
                        FdapConfig,
                        MaybeFdap,
                        UsersConfig,
                    },
                    http::Uri,
                    loga::ResultContext,
                    moka::future::Cache,
                    std::{
                        str::FromStr,
                        time::Duration,
                    },
                };
                use crate::subsystems::accounts::*;

                (|| -> Result<UsersState, loga::Error> {
                    let (users, fdap): (&MaybeFdap<UsersConfig>, &Option<FdapConfig>) =
                        (&config.users, &config.fdap);
                    match users {
                        MaybeFdap::Fdap(subpath) => {
                            let Some(fdap) = fdap else {
                                return Err(
                                    loga::err("User config set to use FDAP but no FDAP configured at config root"),
                                );
                            };
                            return Ok(UsersState::Fdap {
                                client: fdap::Client::builder()
                                    .with_base_url(Uri::from_str(&fdap.url).context("Invalid fdap url")?)
                                    .build()
                                    .map_err(|e| loga::err(e.0))
                                    .context("Error setting up fdap client")?,
                                subpath: subpath.clone(),
                                cache: Cache::builder().time_to_live(Duration::from_secs(10)).build(),
                            });
                        },
                        MaybeFdap::Local(users) => {
                            return Ok(UsersState::Local { users: users.users.keys().cloned().collect() });
                        },
                    }
                })()
            }?;
            let state = Arc::new(State {
                log: log.clone(),
                db: db,
                oidc_state: {
                    use {
                        crate::{
                            cap_fn,
                            interface::config::OidcConfig,
                        },
                        htwrap::url::{
                            new_abs_url,
                            UriJoin,
                        },
                        loga::{
                            ea,
                            Log,
                            ResultContext,
                        },
                        moka::future::Cache,
                        openidconnect::{
                            core::{
                                CoreClient,
                                CoreProviderMetadata,
                            },
                            ClientId,
                            ClientSecret,
                            IssuerUrl,
                            RedirectUrl,
                        },
                        std::time::Duration,
                    };
                    use crate::subsystems::oidc::*;

                    let ret: Result<OidcState, loga::Error> = async {
                        let (log, public_url, oidc_config): (&Log, &str, OidcConfig) =
                            (&log, &config.public_url, config.oidc_config);
                        let log = log.fork(ea!(subsystem = "oidc"));
                        let public_url = new_abs_url(public_url).context("Invalid public url")?;
                        let redirect_url =
                            RedirectUrl::new(
                                public_url
                                    .join("/oidc")
                                    .context("Error building OIDC redirect url from public url")?
                                    .to_string(),
                            ).context("Invalid OIDC redirect url")?;
                        let client =
                            CoreClient::from_provider_metadata(
                                CoreProviderMetadata::discover_async(
                                    IssuerUrl::new(oidc_config.provider_url)?,
                                    &cap_fn!((r)(log) {
                                        return oidc_http_client(&log, r)
                                            .await
                                            .map_err(|e| std::io::Error::other(e.to_string()));
                                    }),
                                ).await?,
                                ClientId::new(oidc_config.client_id.clone()),
                                oidc_config.client_secret.as_ref().map(|s| ClientSecret::new(s.clone())),
                            ).set_redirect_uri(redirect_url);
                        return Ok(OidcState {
                            log: log,
                            public_url: public_url,
                            client: client,
                            pre_sessions: Cache::builder()
                                .max_capacity(10_000)
                                .time_to_live(Duration::from_secs(60 * 10))
                                .build(),
                        });
                    }.await;
                    ret
                }?,
                users_state: users_state,
                s2s_state: {
                    use {
                        moka::future::Cache,
                        spaghettinuum_native::service::resolver::Resolver,
                    };
                    use crate::subsystems::s2s::*;

                    (|| -> S2sState {
                        let (resolver,): (Resolver,) = (spagh_resolver.clone(),);
                        return S2sState {
                            resolver: resolver,
                            challenges: Cache::builder().time_to_live({
                                use std::time::Duration;

                                let value: Duration = Duration::from_secs(30);
                                value
                            }).max_capacity(65536).build(),
                            tokens: Cache::builder().time_to_live({
                                use std::time::Duration;

                                let value: Duration = Duration::from_secs(300);
                                value
                            }).max_capacity(65536).build(),
                        };
                    })()
                },
                s2s_client: Arc::new({
                    use {
                        htwrap::htreq::Limits,
                        loga::Log,
                        moka::future::Cache,
                        spaghettinuum_native::service::resolver::Resolver,
                        std::time::Duration,
                    };
                    use crate::subsystems::s2s_client::*;

                    (|| -> S2sClient {
                        let (log, resolver): (Log, Resolver) = (log.clone(), spagh_resolver.clone());
                        return S2sClient {
                            log: log,
                            resolver: resolver,
                            limits: Limits {
                                read_body_size: 16 * 1024 * 1024,
                                ..Default::default()
                            },
                            tokens: Cache::builder().time_to_live(Duration::from_secs(240)).build(),
                        };
                    })()
                }),
                spagh_publish: {
                    use {
                        crate::interface::s2s::SpaghRecord,
                        spaghettinuum_native::{
                            interface::wire::api::publish::latest::InfoResponse,
                            service::publisher::Publisher,
                        },
                        std::{
                            collections::HashMap,
                            sync::{
                                Arc,
                                RwLock,
                            },
                        },
                    };
                    use crate::subsystems::spagh_publish::*;

                    (|| -> SpaghPublishState {
                        let (publisher, publisher_info, record): (Arc<Publisher>, InfoResponse, SpaghRecord) =
                            (spagh_publisher.clone(), InfoResponse {
                                advertise_addr: spagh_publisher_advertise_addr,
                                cert_pub_hash: spagh_publisher.pub_cert_hash(),
                            }, SpaghRecord::V1(SpaghRecordV1 {
                                ips: global_ips,
                                port: config.s2s_advertise_port.unwrap_or(s2s_bind_sockaddr.port()),
                            }));
                        return SpaghPublishState {
                            publisher: publisher,
                            publisher_info: publisher_info,
                            record: serde_json::to_value(&record).unwrap(),
                            certs: Arc::new(IdentityCerts(RwLock::new(HashMap::new()))),
                        };
                    })()
                },
                ws_bus: Arc::new({
                    use {
                        std::{
                            collections::HashMap,
                            sync::atomic::AtomicU64,
                        },
                        tokio::sync::RwLock,
                    };
                    use crate::subsystems::ws_bus::*;

                    (|| -> WsBus {
                        return WsBus {
                            subscribers: RwLock::new(HashMap::new()),
                            next_id: AtomicU64::new(0),
                        };
                    })()
                }),
                vapid_keys: vapid_keys,
                default_page_size: config.default_page_size,
                pull_scheduler: {
                    use {
                        std::{
                            collections::{
                                BinaryHeap,
                                HashMap,
                            },
                            sync::Arc,
                        },
                        tokio::sync::{
                            Mutex,
                            Notify,
                        },
                    };
                    use crate::subsystems::pull_scheduler::*;

                    (|| -> Arc<PullScheduler> {
                        return Arc::new(PullScheduler {
                            inner: Mutex::new(Inner {
                                queue: BinaryHeap::new(),
                                state: HashMap::new(),
                            }),
                            wake: Notify::new(),
                        });
                    })()
                },
                files_dir: files_dir,
                file_cache_dir: file_cache_dir,
                file_cache_fills: std::sync::Mutex::new(std::collections::HashSet::new()),
                ffmpeg: config.ffmpeg,
                max_file_size: config.max_file_size,
                file_cache_size: config.file_cache_size,
                calendar_cache: moka::sync::Cache::builder()
                    .max_capacity(4096)
                    .time_to_idle(std::time::Duration::from_secs(60 * 60))
                    .build(),
                calendar_page_cache: moka::future::Cache::builder()
                    .max_capacity(4096)
                    .time_to_idle(std::time::Duration::from_secs(60 * 60))
                    .build(),
            });
            {
                use crate::{
                    db,
                    dbutil::tx,
                    State,
                };
                use crate::subsystems::spagh_publish::*;

                let ret: Result<(), loga::Error> = async {
                    let (state,): (&State,) = (&state,);
                    let identities = tx(&state.db, move |db_tx| {
                        return Ok(good_ormning::sqlite::good_query_many!(
                            db,
                            r#"select id from identity where soft_deleted_at is null"#;
                            db_tx
                        ).map_err(|e| loga::err(e.0))?);
                    }).await?;
                    for identity in identities {
                        publish_identity(state, identity.0).await?;
                    }
                    return Ok(());
                }.await;
                ret
            }?;
            tm.periodic("spagh_cert_refresh", {
                use std::time::Duration;

                let value: Duration = Duration::from_secs(60 * 60);
                value
            }, {
                let state = state.clone();
                move || {
                    let state = state.clone();
                    return async move {
                        match {
                            use {
                                crate::{
                                    db,
                                    dbutil::tx,
                                    subsystems::messages::local_signer,
                                    State,
                                },
                                spaghettinuum_native::{
                                    interface::stored::self_tls::RefreshTlsState,
                                    self_tls::request_cert,
                                    utils::{
                                        identity_secret::IdentitySigner,
                                        tls_util::extract_expiry,
                                    },
                                },
                                std::{
                                    sync::{
                                        Arc,
                                        Mutex,
                                    },
                                    time::SystemTime,
                                },
                            };
                            use crate::subsystems::spagh_publish::*;

                            let ret: Result<(), loga::Error> = async {
                                let (state,): (&State,) = (&state,);
                                let rows = tx(&state.db, move |db_tx| {
                                    return Ok(good_ormning::sqlite::good_query_many!(
                                        db,
                                        r#"select id, secret, tls_state from identity where soft_deleted_at is null and tls_state is not null"#;
                                        db_tx
                                    ).map_err(|e| loga::err(e.0))?);
                                }).await?;
                                for row in rows {
                                    let identity = row.id.0;
                                    let RefreshTlsState::V1(mut tls_state) = row.tls_state.unwrap();
                                    let expiry = extract_expiry(tls_state.current.pub_pem.as_bytes())?;
                                    let now = SystemTime::now();
                                    let changed = match tls_state.pending.take() {
                                        Some(pending) => {
                                            if now >= expiry - CERT_SWAP_BUFFER {
                                                tls_state.current = pending;
                                                true
                                            } else {
                                                tls_state.pending = Some(pending);
                                                false
                                            }
                                        },
                                        None => {
                                            if now >= expiry - CERT_SWAP_BUFFER - RECORD_TTL * 2 {
                                                let signer =
                                                    Arc::new(Mutex::new(local_signer(row.secret.0)?)) as
                                                        Arc<Mutex<dyn IdentitySigner>>;
                                                tls_state.pending =
                                                    Some(request_cert(&state.log, signer, cert_options()).await?);
                                                true
                                            } else {
                                                false
                                            }
                                        },
                                    };
                                    if changed {
                                        store_tls_state(state, identity, tls_state.clone()).await?;
                                        set_values(state, identity, &tls_state).await?;
                                    }
                                }
                                return Ok(());
                            }.await;
                            ret
                        } {
                            Ok(_) => { },
                            Err(e) => {
                                state.log.log_err(loga::WARN, e.context("Error refreshing identity TLS certs"));
                            },
                        }
                    };
                }
            });
            {
                use {
                    crate::{
                        db,
                        dbutil::tx,
                        State,
                    },
                    loga::ResultContext,
                    std::sync::Arc,
                };
                use crate::subsystems::pull_scheduler::*;

                let ret: Result<(), loga::Error> = async {
                    let (state,): (&Arc<State>,) = (&state,);
                    let keys = tx(&state.db, move |db_tx| {
                        let channels = good_ormning::sqlite::good_query_many!(
                            db,
                            r#"select account_id, identity, id, local_identity from channel where deleted is null"#;
                            db_tx
                        ).map_err(|e| loga::err(e.0))?;
                        let mut keys = vec![];
                        for c in channels {
                            if c.local_identity.0 == c.identity.0 {
                                for member in {
                                    use {
                                        crate::{
                                            db,
                                            interface::db::{
                                                DbAccountId,
                                                DbChannelId,
                                                DbIdentity,
                                            },
                                        },
                                        rusqlite::Transaction,
                                        spaghettinuum::interface::identity::Identity,
                                    };

                                    (|| -> Result<Vec<Identity>, loga::Error> {
                                        let (db_tx, account, owner, channel):
                                            (&mut db::Db<Transaction<'_>>, DbAccountId, DbIdentity, &DbChannelId) =
                                            (db_tx, c.account_id, c.identity, &c.id);
                                        return Ok(good_ormning::sqlite::good_query_many!(
                                            db,
                                            r#"select member from channel_member where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and can_post = ${bool = true}"#;
                                            db_tx
                                        ).map_err(|e| loga::err(e.0))?.into_iter().map(|r| r.0).collect());
                                    })()
                                }? {
                                    keys.push(PullKey {
                                        account_id: c.account_id.0,
                                        owner: c.identity.0,
                                        channel: c.id.0.clone(),
                                        source: member,
                                    });
                                }
                            } else {
                                keys.push(PullKey {
                                    account_id: c.account_id.0,
                                    owner: c.identity.0,
                                    channel: c.id.0.clone(),
                                    source: c.identity.0,
                                });
                            }
                        }
                        return Ok(keys);
                    }).await.context("Error enumerating channels for scheduler seed")?;
                    for key in keys {
                        state.pull_scheduler.add_or_reset(key).await;
                    }
                    return Ok(());
                }.await;
                ret
            }?;
            tm.critical_task("pull_scheduler", {
                let scheduler = state.pull_scheduler.clone();
                let state = state.clone();
                async move {
                    return {
                        use {
                            crate::{
                                subsystems::messages::{
                                    PullOutcome,
                                },
                                State,
                            },
                            rand::RngExt,
                            std::{
                                cmp::Reverse,
                                sync::Arc,
                                time::Duration,
                            },
                            tokio::time::Instant,
                        };
                        use crate::subsystems::pull_scheduler::*;

                        let ret: Result<(), loga::Error> = async {
                            let (pull_scheduler, state): (Arc<PullScheduler>, Arc<State>) = (scheduler, state);
                            loop {
                                let next_at = {
                                    let mut inner = pull_scheduler.inner.lock().await;
                                    let mut next_at = None;
                                    loop {
                                        let Some(Reverse(top)) = inner.queue.peek() else {
                                            break;
                                        };
                                        let live = match inner.state.get(&top.key) {
                                            Some(s) => s.version == top.version && !s.running,
                                            None => false,
                                        };
                                        if !live {
                                            inner.queue.pop();
                                            continue;
                                        }
                                        if top.at > Instant::now() {
                                            next_at = Some(top.at);
                                            break;
                                        }
                                        let Some(Reverse(entry)) = inner.queue.pop() else {
                                            break;
                                        };
                                        inner.state.get_mut(&entry.key).unwrap().running = true;
                                        let scheduler = pull_scheduler.clone();
                                        let state = state.clone();
                                        tokio::spawn(async move {
                                            let outcome =
                                                match crate::subsystems::messages::pull_and_store(
                                                    &state,
                                                    &entry.key,
                                                ).await {
                                                    Ok(outcome) => outcome,
                                                    Err(e) => {
                                                        state
                                                            .log
                                                            .log_err(
                                                                loga::WARN,
                                                                e.context("Error executing scheduled pull"),
                                                            );
                                                        PullOutcome::NoNew
                                                    },
                                                };
                                            let key = entry.key;
                                            {
                                                let mut inner = scheduler.inner.lock().await;
                                                let Some(s) = inner.state.get_mut(&key) else {
                                                    return;
                                                };
                                                s.running = false;
                                                let backoff = s.backoff;
                                                let rerun = s.rerun;
                                                s.rerun = false;
                                                let next_backoff = match outcome {
                                                    PullOutcome::Gone => {
                                                        inner.state.remove(&key);
                                                        return;
                                                    },
                                                    PullOutcome::New => PULL_MIN_BACKOFF,
                                                    PullOutcome::NoNew => backoff.saturating_mul(2).min({
                                                        use std::time::Duration;

                                                        let value: Duration = Duration::from_secs(3600);
                                                        value
                                                    }),
                                                };
                                                let at = if rerun {
                                                    Instant::now()
                                                } else {
                                                    let frac: f64 =
                                                        rand
                                                        ::rng().random_range(-PULL_JITTER_FRAC .. PULL_JITTER_FRAC);
                                                    let secs = next_backoff.as_secs_f64() * (1.0 + frac);
                                                    Instant::now() + Duration::from_secs_f64(secs.max(1.0))
                                                };
                                                inner.schedule(key, at, next_backoff);
                                            }
                                            scheduler.wake.notify_one();
                                        });
                                    }
                                    next_at
                                };
                                match next_at {
                                    None => {
                                        pull_scheduler.wake.notified().await;
                                    },
                                    Some(at) => {
                                        tokio::select!{
                                            _ = tokio:: time:: sleep_until(at) => {
                                            },
                                            _ = pull_scheduler.wake.notified() => {
                                            },
                                        };
                                    },
                                }
                            }
                        }.await;
                        ret
                    };
                }
            });
            tm.critical_task("backfill", {
                let state = state.clone();
                async move {
                    return {
                        use {
                            crate::{
                                db,
                                dbutil::tx,
                                interface::db::{
                                    DbAccountId,
                                    DbChannelId,
                                    DbIdentity,
                                },
                                State,
                            },
                            jiff::Timestamp,
                            loga::{
                                ea,
                                ResultContext,
                            },
                            shared::interface::shared::QualifiedChannelId,
                            std::sync::Arc,
                        };
                        use crate::subsystems::messages::*;

                        let ret: Result<(), loga::Error> = async {
                            let (state,): (Arc<State>,) = (state,);
                            let mut tick: usize = 0;
                            loop {
                                tokio::time::sleep({
                                    use std::time::Duration;

                                    let value: Duration = Duration::from_secs(2);
                                    value
                                }).await;
                                let candidates = match tx(&state.db, move |db_tx| {
                                    return Ok(good_ormning::sqlite::good_query_many!(
                                        db,
                                        r#"select account_id, identity, id from channel where deleted is null and backfill_done = ${bool = false} and (retention_mode = ${string = RETENTION_MODE_ALL} or retention_mode = ${string = RETENTION_MODE_DAYS})"#;
                                        db_tx
                                    ).map_err(|e| loga::err(e.0))?.into_iter().map(|r| BackfillCandidate {
                                        account_id: r.account_id.0,
                                        channel: QualifiedChannelId {
                                            identity: r.identity.0,
                                            channel: r.id.0,
                                        },
                                    }).collect::<Vec<_>>());
                                }).await {
                                    Ok(c) => c,
                                    Err(e) => {
                                        state
                                            .log
                                            .log_err(loga::WARN, e.context("Error listing backfill candidates"));
                                        continue;
                                    },
                                };
                                if candidates.is_empty() {
                                    continue;
                                }
                                let c = &candidates[tick % candidates.len()];
                                tick = tick.wrapping_add(1);
                                if let Err(e) = async {
                                    flowcontrol::ta_return!((), loga::Error);
                                    let state = &state;
                                    let account = DbAccountId(c.account_id);
                                    let db_owner = DbIdentity(c.channel.identity);
                                    let db_channel_id = DbChannelId(c.channel.channel.clone());
                                    let now = Timestamp::now();
                                    let Some((local, lowest)) = tx(&state.db, {
                                        let db_channel_id = db_channel_id.clone();
                                        move |db_tx| {
                                            let Some(local) =
                                                local_channel_get(db_tx, account, db_owner, &db_channel_id)? else {
                                                    return Ok(None);
                                                };
                                            let lowest = good_ormning::sqlite::good_query_opt!(
                                                db,
                                                r#"select snap_offset, receive_time from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and snap_offset is not null order by snap_offset asc limit ${i64 = 1}"#;
                                                db_tx
                                            ).map_err(|e| loga::err(e.0))?;
                                            return Ok(Some((local, lowest)));
                                        }
                                    }).await.context("Error reading backfill state")? else {
                                        return Ok(());
                                    };
                                    let cutoff = retention_cutoff(now, local.retention)?;
                                    let page_size = local.page_size as u64;
                                    let target_page = match lowest {
                                        Some(row) => {
                                            let offset =
                                                i64_to_u64(
                                                    row.snap_offset.context("Lowest snap row has no snap offset")?,
                                                );
                                            if let Some(cutoff) = cutoff {
                                                if row.receive_time.0 < cutoff {
                                                    mark_backfill_done(state, c).await?;
                                                    return Ok(());
                                                }
                                            }
                                            if let Some(floor) = local.floor_snap_offset {
                                                if offset <= i64_to_u64(floor) {
                                                    mark_backfill_done(state, c).await?;
                                                    return Ok(());
                                                }
                                            }
                                            let page = offset / page_size;
                                            if offset % page_size == 0 {
                                                if page == 0 {
                                                    mark_backfill_done(state, c).await?;
                                                    return Ok(());
                                                }
                                                page - 1
                                            } else {
                                                page
                                            }
                                        },
                                        None => {
                                            let info =
                                                member_refresh_info(state, c.account_id, &c.channel, &local).await?;
                                            let Some(latest) = info.latest_snap_offset else {
                                                return Ok(());
                                            };
                                            latest / page_size
                                        },
                                    };
                                    let Some(fetched) =
                                        member_fetch_snap_page(
                                            state,
                                            c.account_id,
                                            &c.channel,
                                            &local,
                                            target_page,
                                        ).await? else {
                                            mark_backfill_done(state, c).await?;
                                            return Ok(());
                                        };
                                    let originals: Vec<&Verified> =
                                        fetched.iter().filter(|v| v.snap_offset.is_some()).collect();
                                    if target_page == 0 || originals.is_empty() {
                                        mark_backfill_done(state, c).await?;
                                        return Ok(());
                                    }
                                    if let Some(cutoff) = cutoff {
                                        if originals.iter().any(|v| v.receive_time < cutoff) {
                                            mark_backfill_done(state, c).await?;
                                        }
                                    }
                                    return Ok(());
                                }.await {
                                    state
                                        .log
                                        .log_err(
                                            loga::WARN,
                                            e.context_with(
                                                "Error backfilling channel",
                                                ea!(owner = c.channel.identity, channel = c.channel.channel.0),
                                            ),
                                        );
                                }
                            }
                        }.await;
                        ret
                    };
                }
            });
            tm.critical_task("gc", {
                let state = state.clone();
                async move {
                    return {
                        use {
                            crate::{
                                db,
                                dbutil::tx,
                                interface::db::{
                                    DbAccountId,
                                    DbChannelId,
                                    DbIdentity,
                                    DbTimeMs,
                                },
                                State,
                            },
                            jiff::Timestamp,
                            loga::{
                                ea,
                                DebugDisplay,
                                ResultContext,
                            },
                            shared::interface::shared::{
                                FileHash,
                                Message,
                                MessageBlock,
                                MessageBody,
                            },
                            spaghettinuum::interface::identity::Identity,
                            std::{
                                collections::HashSet,
                                sync::Arc,
                                time::Duration,
                            },
                        };
                        use crate::subsystems::messages::*;

                        let ret: Result<(), loga::Error> = async {
                            let (state,): (Arc<State>,) = (state,);
                            loop {
                                let now = Timestamp::now();
                                if let Err(e) = tx(&state.db, move |db_tx| {
                                    let channels = good_ormning::sqlite::good_query_many!(
                                        db,
                                        r#"select account_id, identity, id, retention_mode, retention_secs from channel where deleted is null and retention_mode = ${string = RETENTION_MODE_DAYS}"#;
                                        db_tx
                                    ).map_err(|e| loga::err(e.0))?;
                                    for c in channels {
                                        let Some(cutoff) =
                                            retention_cutoff(
                                                now,
                                                retention_from_db(&c.retention_mode, c.retention_secs),
                                            )? else {
                                                continue;
                                            };
                                        let expired = good_ormning::sqlite::good_query_many!(
                                            db,
                                            r#"select sender, sender_unique, rel_type, rel_target_sender, rel_target_unique from message where account_id = ${account_id_t = &c.account_id} and identity = ${identity_id_t = &c.identity} and channel = ${channel_id_t = &c.id} and receive_time < ${time_ms_t = &crate::interface::db::DbTimeMs(cutoff)}"#;
                                            db_tx
                                        ).map_err(|e| loga::err(e.0))?;
                                        for m in expired {
                                            good_ormning::sqlite::good_query!(
                                                db,
                                                r#"delete from message_file where account_id = ${account_id_t = &c.account_id} and identity = ${identity_id_t = &c.identity} and channel = ${channel_id_t = &c.id} and source = ${string = MESSAGE_FILE_SOURCE_MESSAGE} and sender = ${identity_id_t = &m.sender} and sender_unique = ${i64 = m.sender_unique}"#;
                                                db_tx
                                            ).map_err(|e| loga::err(e.0))?;
                                            good_ormning::sqlite::good_query!(
                                                db,
                                                r#"delete from message where account_id = ${account_id_t = &c.account_id} and identity = ${identity_id_t = &c.identity} and channel = ${channel_id_t = &c.id} and sender = ${identity_id_t = &m.sender} and sender_unique = ${i64 = m.sender_unique}"#;
                                                db_tx
                                            ).map_err(|e| loga::err(e.0))?;
                                            let (root_sender, root_unique) =
                                                match (
                                                    m.rel_type.as_deref(),
                                                    m.rel_target_sender,
                                                    m.rel_target_unique,
                                                ) {
                                                    (
                                                        Some(REL_EDIT_OF),
                                                        Some(target_sender),
                                                        Some(target_unique),
                                                    ) => (
                                                        target_sender,
                                                        target_unique,
                                                    ),
                                                    (Some(REL_EDIT_OF), _, _) => {
                                                        return Err(loga::err("Edit row missing target columns"));
                                                    },
                                                    _ => (m.sender, m.sender_unique),
                                                };
                                            crate::subsystems::calendar::message_event_reindex(
                                                db_tx,
                                                c.account_id,
                                                c.identity,
                                                &c.id,
                                                root_sender,
                                                root_unique,
                                            )?;
                                        }
                                    }
                                    return Ok(());
                                }).await {
                                    state
                                        .log
                                        .log_err(loga::WARN, e.context("Error running message retention cleanup"));
                                }
                                match tx(&state.db, move |db_tx| {
                                    let mut expected = HashSet::new();
                                    for m in good_ormning::sqlite::good_query_many!(
                                        db,
                                        r#"select account_id, identity, channel, sender, sender_unique, blocks from message"#;
                                        db_tx
                                    ).map_err(|e| loga::err(e.0))? {
                                        for block in parse_blocks(&m.blocks)? {
                                            let MessageBlock::File(f) = block else {
                                                continue;
                                            };
                                            expected.insert(
                                                (
                                                    m.account_id.0,
                                                    m.identity.0,
                                                    m.channel.0.clone(),
                                                    MESSAGE_FILE_SOURCE_MESSAGE.to_string(),
                                                    m.sender.0,
                                                    m.sender_unique,
                                                    f.file.hash.0,
                                                ),
                                            );
                                        }
                                    }
                                    for m in good_ormning::sqlite::good_query_many!(
                                        db,
                                        r#"select account_id, identity, channel, sender, sender_unique, raw from outbox"#;
                                        db_tx
                                    ).map_err(|e| loga::err(e.0))? {
                                        let message =
                                            serde_json::from_str::<Message>(
                                                &m.raw,
                                            ).context("Stored outbox message was not valid JSON")?;
                                        let body =
                                            serde_json::from_str::<MessageBody>(
                                                &message.0.message,
                                            ).context("Stored outbox body was not valid JSON")?;
                                        for block in body.blocks {
                                            let MessageBlock::File(f) = block else {
                                                continue;
                                            };
                                            expected.insert(
                                                (
                                                    m.account_id.0,
                                                    m.identity.0,
                                                    m.channel.0.clone(),
                                                    MESSAGE_FILE_SOURCE_OUTBOX.to_string(),
                                                    m.sender.0,
                                                    m.sender_unique,
                                                    f.file.hash.0,
                                                ),
                                            );
                                        }
                                    }
                                    let mut extra = vec![];
                                    for r in good_ormning::sqlite::good_query_many!(
                                        db,
                                        r#"select account_id, identity, channel, source, sender, sender_unique, hash from message_file"#;
                                        db_tx
                                    ).map_err(|e| loga::err(e.0))? {
                                        let key =
                                            (
                                                r.account_id.0,
                                                r.identity.0,
                                                r.channel.0.clone(),
                                                r.source.clone(),
                                                r.sender.0,
                                                r.sender_unique,
                                                r.hash.clone(),
                                            );
                                        if !expected.remove(&key) {
                                            good_ormning::sqlite::good_query!(
                                                db,
                                                r#"delete from message_file where account_id = ${account_id_t = &r.account_id} and identity = ${identity_id_t = &r.identity} and channel = ${channel_id_t = &r.channel} and source = ${string = r.source.as_str()} and sender = ${identity_id_t = &r.sender} and sender_unique = ${i64 = r.sender_unique} and hash = ${string = r.hash.as_str()}"#;
                                                db_tx
                                            ).map_err(|e| loga::err(e.0))?;
                                            extra.push(key);
                                        }
                                    }
                                    let missing = expected.into_iter().collect::<Vec<_>>();
                                    for (account_id, owner, channel, source, sender, sender_unique, hash) in &missing {
                                        good_ormning::sqlite::good_query!(
                                            db,
                                            r#"insert into message_file (account_id, identity, channel, source, sender, sender_unique, hash) values (${account_id_t = &DbAccountId(*account_id)}, ${identity_id_t = &DbIdentity(*owner)}, ${channel_id_t = &DbChannelId(channel.clone())}, ${string = source.as_str()}, ${identity_id_t = &DbIdentity(*sender)}, ${i64 = *sender_unique}, ${string = hash.as_str()})"#;
                                            db_tx
                                        ).map_err(|e| loga::err(e.0))?;
                                    }
                                    return Ok((extra, missing));
                                }).await {
                                    Ok((extra, missing)) => {
                                        for key in extra {
                                            state
                                                .log
                                                .log_with(
                                                    loga::WARN,
                                                    "Deleted message file index entry with no matching message",
                                                    ea!(
                                                        account = key.0.0,
                                                        owner = key.1,
                                                        channel = key.2.0,
                                                        source = key.3,
                                                        sender = key.4,
                                                        sender_unique = key.5,
                                                        hash = key.6
                                                    ),
                                                );
                                        }
                                        for key in missing {
                                            state
                                                .log
                                                .log_with(
                                                    loga::WARN,
                                                    "Added missing message file index entry",
                                                    ea!(
                                                        account = key.0.0,
                                                        owner = key.1,
                                                        channel = key.2.0,
                                                        source = key.3,
                                                        sender = key.4,
                                                        sender_unique = key.5,
                                                        hash = key.6
                                                    ),
                                                );
                                        }
                                    },
                                    Err(e) => {
                                        state
                                            .log
                                            .log_err(loga::WARN, e.context("Error reconciling message file index"));
                                    },
                                }
                                match async {
                                    flowcontrol::ta_return!(Vec <(Identity, String, bool) >, loga::Error);
                                    let rows = tx(&state.db, move |db_tx| {
                                        return Ok(good_ormning::sqlite::good_query_many!(
                                            db,
                                            r#"select owner, hash, thumbnail from file_cache"#;
                                            db_tx
                                        ).map_err(|e| loga::err(e.0))?);
                                    }).await.context("Error listing file cache entries")?;
                                    let mut stale = vec![];
                                    for r in rows {
                                        let path =
                                            crate::subsystems::files::file_cache_path(
                                                &state,
                                                r.owner.0,
                                                &r.hash,
                                                r.thumbnail,
                                            );
                                        if tokio::fs::try_exists(&path)
                                            .await
                                            .context_with("Error checking cached file", ea!(path = path.dbg_str()))? {
                                            continue;
                                        }
                                        tx(&state.db, {
                                            let hash = r.hash.clone();
                                            move |db_tx| {
                                                good_ormning::sqlite::good_query!(
                                                    db,
                                                    r#"delete from file_cache where owner = ${identity_id_t = &r.owner} and hash = ${string = hash.as_str()} and thumbnail = ${bool = r.thumbnail}"#;
                                                    db_tx
                                                ).map_err(|e| loga::err(e.0))?;
                                                return Ok(());
                                            }
                                        }).await.context("Error deleting stale file cache entry")?;
                                        stale.push((r.owner.0, r.hash, r.thumbnail));
                                    }
                                    return Ok(stale);
                                }.await {
                                    Ok(stale) => {
                                        for (owner, hash, thumbnail) in stale {
                                            state
                                                .log
                                                .log_with(
                                                    loga::WARN,
                                                    "Deleted file cache entry whose file was missing",
                                                    ea!(owner = owner, hash = hash, thumbnail = thumbnail),
                                                );
                                        }
                                    },
                                    Err(e) => {
                                        state.log.log_err(loga::WARN, e.context("Error checking file cache"));
                                    },
                                }
                                if let Err(e) = async {
                                    flowcontrol::ta_return!((), loga::Error);
                                    let cutoff =
                                        DbTimeMs(
                                            now
                                                .checked_sub(Duration::from_secs(3600))
                                                .context("File gc grace cutoff out of range")?,
                                        );
                                    let candidates = tx(&state.db, {
                                        let cutoff = cutoff.clone();
                                        move |db_tx| {
                                            return Ok(good_ormning::sqlite::good_query_many!(
                                                db,
                                                r#"select account_id, hash from file where created < ${time_ms_t = &cutoff}"#;
                                                db_tx
                                            ).map_err(|e| loga::err(e.0))?);
                                        }
                                    }).await.context("Error listing file gc candidates")?;
                                    for c in candidates {
                                        let account = c.account_id;
                                        let hash = c.hash;
                                        let removed = tx(&state.db, {
                                            let hash = hash.clone();
                                            let cutoff = cutoff.clone();
                                            move |db_tx| {
                                                if crate::subsystems::files::file_referenced(db_tx, account, &hash)? {
                                                    return Ok(false);
                                                }
                                                good_ormning::sqlite::good_query!(
                                                    db,
                                                    r#"delete from file where account_id = ${account_id_t = &account} and hash = ${string = hash.as_str()} and created < ${time_ms_t = &cutoff}"#;
                                                    db_tx
                                                ).map_err(|e| loga::err(e.0))?;
                                                return Ok(db_tx.0.changes() > 0);
                                            }
                                        }).await.context("Error deleting unreferenced file row")?;
                                        if !removed {
                                            continue;
                                        }
                                        let path =
                                            crate::subsystems::files::file_path(&state, account, &FileHash(hash));
                                        crate::fsutil::remove_file_if_exists(&path).await?;
                                        crate::fsutil::remove_file_if_exists(
                                            &crate::subsystems::files::thumbnail_path(&path),
                                        ).await?;
                                    }
                                    return Ok(());
                                }.await {
                                    state.log.log_err(loga::WARN, e.context("Error running file gc"));
                                }
                                tokio::time::sleep({
                                    use std::time::Duration;

                                    let value: Duration = Duration::from_secs(3600);
                                    value
                                }).await;
                            }
                        }.await;
                        ret
                    };
                }
            });
            tm.critical_task("reminders", {
                let state = state.clone();
                async move {
                    use {
                        crate::{
                            dbutil::tx,
                            interface::db::DbTimeMs,
                            subsystems::{
                                calendar::events_overlapping,
                                push::push_send,
                            },
                        },
                        jiff::{
                            SignedDuration,
                            Timestamp,
                            ToSpan,
                        },
                        shared::{
                            calendar::{
                                event_occurrences,
                                CalendarWindow,
                                Occurrence,
                            },
                            interface::{
                                shared::{
                                    MessageId,
                                    QualifiedChannelId,
                                    QualifiedMessageId,
                                },
                                wire::s2c::{
                                    PushPayload,
                                    ReminderNotification,
                                },
                            },
                        },
                    };

                    loop {
                        let now = Timestamp::now();
                        match tx(&state.db, move |db_tx| {
                            let channels = good_ormning::sqlite::good_query_many!(
                                db,
                                r#"select account_id, identity, id, reminder_lead_minutes from channel where deleted is null and reminders = ${bool = true}"#;
                                db_tx
                            ).map_err(|e| loga::err(e.0))?;
                            let mut due = vec![];
                            for c in channels {
                                let lead = SignedDuration::from_mins(c.reminder_lead_minutes.max(0));
                                let end = now.checked_add(lead).map_err(|e| loga::err(e.to_string()))?;
                                let utc_now = now.to_zoned(jiff::tz::TimeZone::UTC).date();
                                let utc_end =
                                    end
                                        .to_zoned(jiff::tz::TimeZone::UTC)
                                        .date()
                                        .checked_add(1.day())
                                        .map_err(|e| loga::err(e.to_string()))?;
                                let window = CalendarWindow {
                                    start: now,
                                    end: end
                                        .checked_add(SignedDuration::from_secs(1))
                                        .map_err(|e| loga::err(e.to_string()))?,
                                    start_date: utc_now,
                                    end_date: utc_end,
                                };
                                for e in events_overlapping(
                                    db_tx,
                                    c.account_id,
                                    c.identity,
                                    &c.id,
                                    window.start,
                                    window.end,
                                )? {
                                    for o in event_occurrences(&e.event, &window).map_err(loga::err)? {
                                        let Occurrence::Timed { start, .. } = o else {
                                            continue;
                                        };
                                        if start <= now || start > end {
                                            continue;
                                        }
                                        let root_sender = crate::interface::db::DbIdentity(e.root_sender);
                                        good_ormning::sqlite::good_query!(
                                            db,
                                            r#"insert into reminder_log (account_id, identity, channel, root_sender, root_unique, occurrence_start) values (${account_id_t = &c.account_id}, ${identity_id_t = &c.identity}, ${channel_id_t = &c.id}, ${identity_id_t = &root_sender}, ${i64 = e.root_unique}, ${time_ms_t = &DbTimeMs(start)}) on conflict do nothing"#;
                                            db_tx
                                        ).map_err(|e| loga::err(e.0))?;
                                        if db_tx.0.changes() == 0 {
                                            continue;
                                        }
                                        let version =
                                            good_ormning::sqlite::good_query_opt!(
                                                db,
                                                r#"select blocks from message where account_id = ${account_id_t = &c.account_id} and identity = ${identity_id_t = &c.identity} and channel = ${channel_id_t = &c.id} and seq = ${i64 = e.version_seq}"#;
                                                db_tx
                                            )
                                                .map_err(|e| loga::err(e.0))?
                                                .expect("Indexed event's message version is missing");
                                        let mut texts = vec![];
                                        for block in crate::subsystems::messages::parse_blocks(&version)? {
                                            if let shared::interface::shared::MessageBlock::Text(t) = block {
                                                texts.push(t);
                                            }
                                        }
                                        due.push((c.account_id.0, ReminderNotification {
                                            message: QualifiedMessageId {
                                                channel: QualifiedChannelId {
                                                    identity: c.identity.0,
                                                    channel: c.id.0.clone(),
                                                },
                                                message: MessageId {
                                                    identity: e.root_sender,
                                                    unique: crate::subsystems::messages::i64_to_u64(e.root_unique),
                                                },
                                            },
                                            text: texts.join("\n\n"),
                                            start: start,
                                        }));
                                    }
                                }
                            }
                            let prune_before =
                                now
                                    .checked_sub(SignedDuration::from_hours(24))
                                    .map_err(|e| loga::err(e.to_string()))?;
                            good_ormning::sqlite::good_query!(
                                db,
                                r#"delete from reminder_log where occurrence_start < ${time_ms_t = &DbTimeMs(prune_before)}"#;
                                db_tx
                            ).map_err(|e| loga::err(e.0))?;
                            return Ok(due);
                        }).await {
                            Ok(due) => {
                                for (account_id, reminder) in due {
                                    let payload = serde_json::to_vec(&PushPayload::Reminder(reminder)).unwrap();
                                    if let Err(e) = push_send(&state, account_id, &payload).await {
                                        state.log.log_err(loga::WARN, e.context("Reminder push delivery failed"));
                                    }
                                }
                            },
                            Err(e) => {
                                state.log.log_err(loga::WARN, e.context("Error computing due reminders"));
                            },
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    }
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
                                                            return Ok(s2s_respond::<StartIdentify>(state, Ok({
                                                                use {
                                                                    crate::interface::s2s::{
                                                                        StartIdentify,
                                                                        StartIdentifyRes,
                                                                    },
                                                                    rand::Rng,
                                                                    spaghettinuum::byteszb32::BytesZb32,
                                                                };
                                                                use crate::subsystems::s2s::*;

                                                                let ret: StartIdentifyRes = async {
                                                                    let (s2s_state, req,):
                                                                        (&S2sState, StartIdentify,) =
                                                                        (&(state.s2s_state), r,);
                                                                    let mut buf = [0u8; 32];
                                                                    rand::rng().fill_bytes(&mut buf);
                                                                    s2s_state
                                                                        .challenges
                                                                        .insert(buf.to_vec(), req.identity)
                                                                        .await;
                                                                    return StartIdentifyRes {
                                                                        challenge: BytesZb32(buf.to_vec())
                                                                    };
                                                                }.await;
                                                                ret
                                                            })));
                                                        },
                                                        s2sv1::Req::Identify(r) => {
                                                            return Ok(s2s_respond::<Identify>(state, {
                                                                use {
                                                                    crate::{
                                                                        db,
                                                                        interface::{
                                                                            db::DbIdentity,
                                                                            s2s::{
                                                                                Identify,
                                                                                IdentifyChallenge,
                                                                                IdentifyRes,
                                                                            },
                                                                        },
                                                                    },
                                                                    crate::subsystems::s2s_error::{
                                                                        s2s_tx,
                                                                        S2sError,
                                                                    },
                                                                    deadpool_sqlite::Pool,
                                                                    loga::ResultContext,
                                                                    rand::Rng,
                                                                };
                                                                use crate::subsystems::s2s::*;

                                                                let ret: Result<IdentifyRes, S2sError> = async {
                                                                    let (s2s_state, db, req,):
                                                                        (&S2sState, &Pool, Identify,) =
                                                                        (&(state.s2s_state), &state.db, r,);
                                                                    let unverified =
                                                                        match serde_json
                                                                        ::from_str::<IdentifyChallenge>(
                                                                            &req.challenge.message
                                                                        ) {
                                                                            Ok(v) => v,
                                                                            Err(e) => return Err(
                                                                                S2sError::BadRequest(
                                                                                    format!(
                                                                                        "Identify challenge body is not valid JSON: {}",
                                                                                        e
                                                                                    )
                                                                                )
                                                                            ),
                                                                        };
                                                                    let Some(expected_identity) =
                                                                        s2s_state
                                                                            .challenges
                                                                            .remove(&unverified.challenge.0)
                                                                            .await else {
                                                                            return Err(
                                                                                S2sError::Unauthorized(
                                                                                    "No pending challenge matching response".to_string()
                                                                                )
                                                                            );
                                                                        };
                                                                    if expected_identity != req.identity {
                                                                        return Err(
                                                                            S2sError::Unauthorized(
                                                                                "Challenge was issued to a different identity".to_string()
                                                                            )
                                                                        );
                                                                    }
                                                                    let db_audience = DbIdentity(unverified.audience);
                                                                    let audience_hosted =
                                                                        s2s_tx(
                                                                            db,
                                                                            "checking identify audience",
                                                                            move |db_tx| {
                                                                                return Ok(
                                                                                    good_ormning
                                                                                    ::sqlite
                                                                                    ::good_query_opt!(
                                                                                        db,
                                                                                        r#"select 1 as x from identity where id = ${identity_id_t = &db_audience} and soft_deleted_at is null"#;
                                                                                        db_tx
                                                                                    ).map_err(|e| loga::err(e.0))?
                                                                                );
                                                                            }
                                                                        ).await?;
                                                                    if audience_hosted.is_none() {
                                                                        return Err(
                                                                            S2sError::NotFound(
                                                                                "Identify audience is not hosted on this server".to_string()
                                                                            )
                                                                        );
                                                                    }
                                                                    let Some(kvs) =
                                                                        s2s_state
                                                                            .resolver
                                                                            .get(&req.identity, vec![])
                                                                            .await
                                                                            .context(
                                                                                "Error resolving identity announcement"
                                                                            )? else {
                                                                            return Err(
                                                                                S2sError::BadRequest(
                                                                                    "Identity has no announcement".to_string()
                                                                                )
                                                                            );
                                                                        };
                                                                    let received =
                                                                        match req
                                                                            .challenge
                                                                            .verify(
                                                                                &req.identity,
                                                                                &kvs.announcement.public_key_root,
                                                                                &kvs.announcement.public_key_chain
                                                                            ) {
                                                                            Ok(v) => v,
                                                                            Err(e) => return Err(
                                                                                S2sError::Unauthorized(
                                                                                    format!(
                                                                                        "Signature verification failed: {}",
                                                                                        e
                                                                                    )
                                                                                )
                                                                            ),
                                                                        };
                                                                    if received.challenge.0 != unverified.challenge.0 ||
                                                                        received.audience != unverified.audience {
                                                                        return Err(
                                                                            S2sError::BadRequest(
                                                                                "Verified challenge doesn't match parsed challenge".to_string()
                                                                            )
                                                                        );
                                                                    }
                                                                    let mut token_buf = [0u8; 32];
                                                                    rand::rng().fill_bytes(&mut token_buf);
                                                                    let token = hex::encode(&token_buf);
                                                                    s2s_state
                                                                        .tokens
                                                                        .insert(token.clone(), req.identity)
                                                                        .await;
                                                                    return Ok(IdentifyRes { token: token });
                                                                }.await;
                                                                ret
                                                            }));
                                                        },
                                                        s2sv1::Req::Notify(r) => {
                                                            return Ok(s2s_respond::<Notify>(state, {
                                                                use {
                                                                    crate::{
                                                                        db,
                                                                        interface::{
                                                                            db::{
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            s2s::{
                                                                                Notify,
                                                                                NotifyTimestamp,
                                                                            },
                                                                        },
                                                                        subsystems::{
                                                                            pull_scheduler::PullKey,
                                                                            s2s_error::{
                                                                                s2s_tx,
                                                                                S2sError,
                                                                            },
                                                                        },
                                                                        State,
                                                                    },
                                                                    jiff::Timestamp,
                                                                    loga::ResultContext,
                                                                    std::sync::Arc,
                                                                };
                                                                use crate::subsystems::messages::*;

                                                                let ret: Result<(), S2sError> = async {
                                                                    let (state, req,): (&Arc<State>, Notify,) =
                                                                        (state, r,);
                                                                    let signer = req.signer;
                                                                    let unverified =
                                                                        match serde_json::from_str::<NotifyTimestamp>(
                                                                            &req.signed.message
                                                                        ) {
                                                                            Ok(v) => v,
                                                                            Err(e) => return Err(
                                                                                S2sError::BadRequest(
                                                                                    format!(
                                                                                        "Notify body is not valid JSON: {}",
                                                                                        e
                                                                                    )
                                                                                )
                                                                            ),
                                                                        };
                                                                    let owner = unverified.owner;
                                                                    let channel_id = unverified.channel.clone();
                                                                    let local_rows =
                                                                        s2s_tx(
                                                                            &state.db,
                                                                            "looking up local channel for notify",
                                                                            {
                                                                                let db_owner = DbIdentity(owner);
                                                                                let db_channel_id =
                                                                                    DbChannelId(channel_id.clone());
                                                                                move |db_tx| {
                                                                                    return Ok(
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query_many!(
                                                                                            db,
                                                                                            r#"select account_id, local_identity, public_post from channel where identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id} and deleted is null"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?
                                                                                    );
                                                                                }
                                                                            }
                                                                        ).await?;
                                                                    if local_rows.is_empty() {
                                                                        return Ok(());
                                                                    }
                                                                    let Some(announcement) =
                                                                        state
                                                                            .s2s_state
                                                                            .resolver()
                                                                            .get(&signer, vec![])
                                                                            .await
                                                                            .context(
                                                                                "Error resolving notify signer announcement"
                                                                            )?
                                                                            .map(|kvs| kvs.announcement) else {
                                                                            return Err(
                                                                                S2sError::BadRequest(
                                                                                    "Notify signer has no announcement".to_string()
                                                                                )
                                                                            );
                                                                        };
                                                                    let ts_body =
                                                                        match req
                                                                            .signed
                                                                            .verify(
                                                                                &signer,
                                                                                &announcement.public_key_root,
                                                                                &announcement.public_key_chain
                                                                            ) {
                                                                            Ok(v) => v,
                                                                            Err(e) => return Err(
                                                                                S2sError::Unauthorized(
                                                                                    format!(
                                                                                        "Notify signature verification failed: {}",
                                                                                        e
                                                                                    )
                                                                                )
                                                                            ),
                                                                        };
                                                                    let now = Timestamp::now();
                                                                    let age =
                                                                        now.as_second() -
                                                                            ts_body.timestamp.as_second();
                                                                    if age >
                                                                        crate::interface::s2s::NOTIFY_MAX_AGE_SECS ||
                                                                        age <
                                                                            -crate
                                                                            ::interface
                                                                            ::s2s
                                                                            ::NOTIFY_MAX_AGE_SECS {
                                                                        return Err(
                                                                            S2sError::BadRequest(
                                                                                format!(
                                                                                    "Notify timestamp is outside the freshness window ({}s)",
                                                                                    age
                                                                                )
                                                                            )
                                                                        );
                                                                    }
                                                                    if ts_body.owner != owner ||
                                                                        ts_body.channel != channel_id {
                                                                        return Err(
                                                                            S2sError::BadRequest(
                                                                                "Verified notify body doesn't match parsed body".to_string()
                                                                            )
                                                                        );
                                                                    }
                                                                    let (scheduled, one_shot) =
                                                                        s2s_tx(
                                                                            &state.db,
                                                                            "checking notify signer access",
                                                                            {
                                                                                let channel_id = channel_id.clone();
                                                                                move |db_tx| {
                                                                                    let db_owner = DbIdentity(owner);
                                                                                    let db_channel_id =
                                                                                        DbChannelId(
                                                                                            channel_id.clone()
                                                                                        );
                                                                                    let mut scheduled = vec![];
                                                                                    let mut one_shot = vec![];
                                                                                    for row in local_rows {
                                                                                        let key = PullKey {
                                                                                            account_id: row
                                                                                                .account_id
                                                                                                .0,
                                                                                            owner: owner,
                                                                                            channel: channel_id.clone(),
                                                                                            source: signer,
                                                                                        };
                                                                                        if row.local_identity.0 ==
                                                                                            owner {
                                                                                            if member_can_post(
                                                                                                db_tx,
                                                                                                row.account_id,
                                                                                                db_owner,
                                                                                                &db_channel_id,
                                                                                                DbIdentity(signer)
                                                                                            )? {
                                                                                                scheduled.push(key);
                                                                                            } else if row.public_post {
                                                                                                one_shot.push(key);
                                                                                            }
                                                                                        } else if signer == owner {
                                                                                            scheduled.push(key);
                                                                                        }
                                                                                    }
                                                                                    return Ok((scheduled, one_shot));
                                                                                }
                                                                            }
                                                                        ).await?;
                                                                    if scheduled.is_empty() && one_shot.is_empty() {
                                                                        return Err(
                                                                            S2sError::Forbidden(
                                                                                "Notify signer is not a participant of the referenced channel".to_string()
                                                                            )
                                                                        );
                                                                    }
                                                                    for key in scheduled {
                                                                        state
                                                                            .pull_scheduler
                                                                            .trigger_immediate(key)
                                                                            .await;
                                                                    }
                                                                    for key in one_shot {
                                                                        let state = state.clone();
                                                                        tokio::spawn(async move {
                                                                            if let Err(e) =
                                                                                pull_and_store(&state, &key).await {
                                                                                state
                                                                                    .log
                                                                                    .log_err(
                                                                                        loga::WARN,
                                                                                        e.context(
                                                                                            "Error pulling from public poster"
                                                                                        )
                                                                                    );
                                                                            }
                                                                        });
                                                                    }
                                                                    return Ok(());
                                                                }.await;
                                                                ret
                                                            }));
                                                        },
                                                        s2sv1::Req::Join(r) => {
                                                            return Ok(s2s_respond::<Join>(state, {
                                                                use {
                                                                    crate::{
                                                                        db,
                                                                        interface::{
                                                                            db::{
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            s2s::ChannelInfoRes,
                                                                        },
                                                                        subsystems::{
                                                                            pull_scheduler::PullKey,
                                                                            s2s_error::{
                                                                                not_found,
                                                                                s2s_tx,
                                                                                S2sError,
                                                                            },
                                                                        },
                                                                        State,
                                                                    },
                                                                    std::sync::Arc,
                                                                };
                                                                use crate::subsystems::messages::*;

                                                                let ret:
                                                                    Result<crate::interface::s2s::JoinRes, S2sError> =
                                                                    async {
                                                                        let (state, token, req,):
                                                                            (
                                                                                &Arc<State>,
                                                                                &str,
                                                                                crate::interface::s2s::Join,
                                                                            ) =
                                                                            (state, &auth_token, r,);

                                                                        use crate::interface::s2s::{
                                                                            ChannelOrIdentity,
                                                                            InvitationToken,
                                                                        };

                                                                        let caller =
                                                                            require_caller(state, token).await?;
                                                                        match (req.target, req.invitation) {
                                                                            (
                                                                                ChannelOrIdentity::Channel(channel_id),
                                                                                InvitationToken::Channel(invite)
                                                                            ) => {
                                                                                let db_channel_id =
                                                                                    DbChannelId(channel_id.clone());
                                                                                let db_caller = DbIdentity(caller);
                                                                                let invite_str = invite.0.clone();
                                                                                let (account_id, owner, info) =
                                                                                    s2s_tx(
                                                                                        &state.db,
                                                                                        "redeeming channel invitation",
                                                                                        move |db_tx| {
                                                                                            let invite_row =
                                                                                                lookup_channel_invite(
                                                                                                    db_tx,
                                                                                                    &db_channel_id,
                                                                                                    &invite_str
                                                                                                )?;
                                                                                            let account_id =
                                                                                                invite_row.account_id;
                                                                                            let owner =
                                                                                                invite_row.owner;
                                                                                            let local =
                                                                                                not_found(
                                                                                                    local_channel_get(
                                                                                                        db_tx,
                                                                                                        account_id,
                                                                                                        owner,
                                                                                                        &db_channel_id
                                                                                                    )?,
                                                                                                    "Invited channel no longer exists",
                                                                                                )?;
                                                                                            if !local.is_owner(
                                                                                                owner.0
                                                                                            ) {
                                                                                                return Err(
                                                                                                    S2sError::NotFound(
                                                                                                        "Invited channel isn't owned by the inviting identity".to_string()
                                                                                                    ),
                                                                                                );
                                                                                            }
                                                                                            if caller == owner.0 {
                                                                                                return Err(
                                                                                                    S2sError
                                                                                                    ::BadRequest(
                                                                                                        "Channel owner can't join their own channel".to_string()
                                                                                                    )
                                                                                                );
                                                                                            }
                                                                                            good_ormning
                                                                                            ::sqlite
                                                                                            ::good_query!(
                                                                                                db,
                                                                                                r#"insert into channel_member (account_id, identity, channel, member, confirmed, can_post, memo_short, memo_long) values (${account_id_t = &account_id}, ${identity_id_t = &owner}, ${channel_id_t = &db_channel_id}, ${identity_id_t = &db_caller}, ${bool = true}, ${bool = invite_row.can_post}, ${string = ""}, ${string = ""}) on conflict (account_id, identity, channel, member) do update set confirmed = true"#;
                                                                                                db_tx
                                                                                            ).map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?;
                                                                                            let new_count =
                                                                                                invite_row.used_count +
                                                                                                    1;
                                                                                            good_ormning
                                                                                            ::sqlite
                                                                                            ::good_query!(
                                                                                                db,
                                                                                                r#"update channel_invite set used_count = ${i64 = new_count} where channel = ${channel_id_t = &db_channel_id} and token = ${string = invite_str.as_str()}"#;
                                                                                                db_tx
                                                                                            ).map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?;
                                                                                            let info =
                                                                                                channel_info_res(
                                                                                                    db_tx,
                                                                                                    account_id,
                                                                                                    owner,
                                                                                                    &db_channel_id,
                                                                                                    &local
                                                                                                )?;
                                                                                            return Ok(
                                                                                                (
                                                                                                    account_id.0,
                                                                                                    owner.0,
                                                                                                    info
                                                                                                )
                                                                                            );
                                                                                        }
                                                                                    ).await?;
                                                                                state
                                                                                    .pull_scheduler
                                                                                    .add_or_reset(PullKey {
                                                                                        account_id: account_id,
                                                                                        owner: owner,
                                                                                        channel: channel_id.clone(),
                                                                                        source: caller,
                                                                                    })
                                                                                    .await;
                                                                                return Ok(
                                                                                    crate::interface::s2s::JoinRes {
                                                                                        channel: channel_id,
                                                                                        info: info,
                                                                                    }
                                                                                );
                                                                            },
                                                                            (
                                                                                ChannelOrIdentity::Identity(identity),
                                                                                InvitationToken::Identity(invite)
                                                                            ) => {
                                                                                let db_identity =
                                                                                    DbIdentity(identity);
                                                                                let db_caller = DbIdentity(caller);
                                                                                let invite_str = invite.0.clone();
                                                                                let dm_channel_id =
                                                                                    crate
                                                                                    ::subsystems
                                                                                    ::c2s
                                                                                    ::random_channel_id();
                                                                                let db_dm_channel_id =
                                                                                    DbChannelId(
                                                                                        dm_channel_id.clone()
                                                                                    );
                                                                                let page_size =
                                                                                    state.default_page_size;
                                                                                let page_size_i64 = page_size as i64;
                                                                                if caller == identity {
                                                                                    return Err(
                                                                                        S2sError::BadRequest(
                                                                                            "Can't redeem your own identity invitation".to_string()
                                                                                        )
                                                                                    );
                                                                                }
                                                                                let account_id =
                                                                                    s2s_tx(
                                                                                        &state.db,
                                                                                        "redeeming identity invitation",
                                                                                        move |db_tx| {
                                                                                            let invite_row =
                                                                                                lookup_identity_invite(
                                                                                                    db_tx,
                                                                                                    db_identity,
                                                                                                    &invite_str
                                                                                                )?;
                                                                                            let account_id =
                                                                                                invite_row.account_id;
                                                                                            good_ormning
                                                                                            ::sqlite
                                                                                            ::good_query!(
                                                                                                db,
                                                                                                r#"insert into contact (account_id, id, name, description) values (${account_id_t = &account_id}, ${identity_id_t = &db_caller}, ${string = ""}, ${string = ""}) on conflict do nothing"#;
                                                                                                db_tx
                                                                                            ).map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?;
                                                                                            let dm_idem =
                                                                                                format!(
                                                                                                    "dm:{}:{}",
                                                                                                    caller,
                                                                                                    dm_channel_id.0
                                                                                                );
                                                                                            let new_count =
                                                                                                invite_row.used_count +
                                                                                                    1;
                                                                                            let dm_name =
                                                                                                format!(
                                                                                                    "{} - {}",
                                                                                                    invite_row.name,
                                                                                                    new_count
                                                                                                );
                                                                                            crate
                                                                                            ::subsystems
                                                                                            ::c2s
                                                                                            ::insert_channel_row(
                                                                                                db_tx,
                                                                                                account_id,
                                                                                                db_identity,
                                                                                                &db_dm_channel_id,
                                                                                                db_identity,
                                                                                                &dm_idem,
                                                                                                None,
                                                                                                &dm_name,
                                                                                                &invite_row.description,
                                                                                                None,
                                                                                                None,
                                                                                                None,
                                                                                                page_size_i64,
                                                                                                true,
                                                                                            )?;
                                                                                            good_ormning
                                                                                            ::sqlite
                                                                                            ::good_query!(
                                                                                                db,
                                                                                                r#"insert into channel_member (account_id, identity, channel, member, confirmed, can_post, memo_short, memo_long) values (${account_id_t = &account_id}, ${identity_id_t = &db_identity}, ${channel_id_t = &db_dm_channel_id}, ${identity_id_t = &db_caller}, ${bool = true}, ${bool = true}, ${string = ""}, ${string = ""})"#;
                                                                                                db_tx
                                                                                            ).map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?;
                                                                                            good_ormning
                                                                                            ::sqlite
                                                                                            ::good_query!(
                                                                                                db,
                                                                                                r#"update identity_invite set used_count = ${i64 = new_count} where identity = ${identity_id_t = &db_identity} and token = ${string = invite_str.as_str()}"#;
                                                                                                db_tx
                                                                                            ).map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?;
                                                                                            return Ok(account_id.0);
                                                                                        }
                                                                                    ).await?;
                                                                                state
                                                                                    .pull_scheduler
                                                                                    .add_or_reset(PullKey {
                                                                                        account_id: account_id,
                                                                                        owner: identity,
                                                                                        channel: dm_channel_id.clone(),
                                                                                        source: caller,
                                                                                    })
                                                                                    .await;
                                                                                return Ok(
                                                                                    crate::interface::s2s::JoinRes {
                                                                                        channel: dm_channel_id,
                                                                                        info: ChannelInfoRes {
                                                                                            page_size: u32::try_from(
                                                                                                page_size
                                                                                            ).unwrap_or(u32::MAX),
                                                                                            latest_seq: None,
                                                                                            latest_snap_offset: None,
                                                                                            earliest_snap_offset: None,
                                                                                        },
                                                                                    }
                                                                                );
                                                                            },
                                                                            _ => return Err(
                                                                                S2sError::BadRequest(
                                                                                    "Target/invitation type mismatch".to_string()
                                                                                )
                                                                            ),
                                                                        }
                                                                    }.await;
                                                                ret
                                                            }));
                                                        },
                                                        s2sv1::Req::PullOutbox(r) => {
                                                            return Ok(s2s_respond::<PullOutbox>(state, {
                                                                use {
                                                                    crate::{
                                                                        db,
                                                                        interface::{
                                                                            db::{
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            s2s::{
                                                                                PullOutbox,
                                                                                PulledMessage,
                                                                                PulledMessages,
                                                                            },
                                                                        },
                                                                        subsystems::s2s_error::{
                                                                            not_found,
                                                                            s2s_tx,
                                                                            S2sError,
                                                                        },
                                                                        State,
                                                                    },
                                                                    loga::ResultContext,
                                                                    std::sync::Arc,
                                                                };
                                                                use crate::subsystems::messages::*;

                                                                let ret: Result<PulledMessages, S2sError> = async {
                                                                    let (state, token, req,):
                                                                        (&Arc<State>, &str, PullOutbox,) =
                                                                        (state, &auth_token, r,);
                                                                    let caller = require_caller(state, token).await?;
                                                                    let owner = req.channel.identity;
                                                                    let host = req.host;
                                                                    let db_owner = DbIdentity(owner);
                                                                    let db_host = DbIdentity(host);
                                                                    let db_channel_id =
                                                                        DbChannelId(req.channel.channel.clone());
                                                                    let after =
                                                                        req.after.map(u64_to_i64).unwrap_or(-1);
                                                                    let limit = req.limit.min(PULL_BATCH_LIMIT) as i64;
                                                                    if caller != owner {
                                                                        return Err(
                                                                            S2sError::Forbidden(
                                                                                "Only the channel owner can pull a member's outbox".to_string()
                                                                            )
                                                                        );
                                                                    }
                                                                    return s2s_tx(
                                                                        &state.db,
                                                                        "querying outbox for pull",
                                                                        move |db_tx| {
                                                                            let host_account =
                                                                                not_found(
                                                                                    hosted_identity_account(
                                                                                        db_tx,
                                                                                        db_host
                                                                                    )?,
                                                                                    "Pull host identity is not hosted on this server"
                                                                                )?;
                                                                            let local =
                                                                                not_found(
                                                                                    local_channel_get(
                                                                                        db_tx,
                                                                                        host_account,
                                                                                        db_owner,
                                                                                        &db_channel_id
                                                                                    )?,
                                                                                    "Channel not found on this server",
                                                                                )?;
                                                                            if local.local_identity != host {
                                                                                return Err(
                                                                                    S2sError::BadRequest(
                                                                                        "Pull host doesn't participate in the channel as the requested identity".to_string(),
                                                                                    ),
                                                                                );
                                                                            }
                                                                            let rows =
                                                                                good_ormning::sqlite::good_query_many!(
                                                                                    db,
                                                                                    r#"select sender_unique, created, raw from outbox where account_id = ${account_id_t = &host_account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and sender = ${identity_id_t = &db_host} and sender_unique > ${i64 = after} order by sender_unique limit ${i64 = limit}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            let mut messages =
                                                                                Vec::with_capacity(rows.len());
                                                                            for r in rows {
                                                                                messages.push(PulledMessage {
                                                                                    seq: i64_to_u64(r.sender_unique),
                                                                                    snap_offset: None,
                                                                                    receive_time: r.created.0,
                                                                                    message: serde_json::from_str(
                                                                                        &r.raw
                                                                                    ).context(
                                                                                        "Stored outbox message was not valid JSON"
                                                                                    )?,
                                                                                });
                                                                            }
                                                                            return Ok(
                                                                                PulledMessages { messages: messages }
                                                                            );
                                                                        }
                                                                    ).await;
                                                                }.await;
                                                                ret
                                                            }));
                                                        },
                                                    }
                                                }.await;
                                            }
                                            if head.method == Method::GET && !rest.is_empty() {
                                                return Ok(async {
                                                    let head = &head;

                                                    use crate::{
                                                        interface::s2s::*,
                                                        subsystems::s2s_error::S2sError,
                                                    };

                                                    let auth_token = auth_token(head);
                                                    let first = rest.split('/').next().unwrap();
                                                    match first {
                                                        GET_PREFIX_CHANNEL_INFO => {
                                                            let req =
                                                                match <GetChannelInfo as c2s
                                                                ::PathReqTrait>::deserialize_path(
                                                                    rest
                                                                ) {
                                                                    Ok(r) => r,
                                                                    Err(e) => return s2s_error_response(
                                                                        state,
                                                                        S2sError::BadRequest(e)
                                                                    ),
                                                                };
                                                            return s2s_respond_get::<GetChannelInfo>(state, {
                                                                use {
                                                                    crate::{
                                                                        interface::{
                                                                            db::{
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            s2s::{
                                                                                ChannelInfoRes,
                                                                                GetChannelInfo,
                                                                            },
                                                                        },
                                                                        subsystems::s2s_error::{
                                                                            not_found,
                                                                            s2s_tx,
                                                                            Cached,
                                                                            S2sError,
                                                                        },
                                                                        State,
                                                                    },
                                                                    std::sync::Arc,
                                                                };
                                                                use crate::subsystems::messages::*;

                                                                let ret: Result<Cached<ChannelInfoRes>, S2sError> =
                                                                    async {
                                                                        let (state, token, req,):
                                                                            (&Arc<State>, &str, GetChannelInfo,) =
                                                                            (state, &auth_token, req,);
                                                                        let caller =
                                                                            resolve_caller(state, token).await?;
                                                                        let db_owner =
                                                                            DbIdentity(req.channel.identity);
                                                                        let db_channel_id =
                                                                            DbChannelId(req.channel.channel);
                                                                        return s2s_tx(
                                                                            &state.db,
                                                                            "reading channel info",
                                                                            move |db_tx| {
                                                                                let (account, local) =
                                                                                    not_found(
                                                                                        owner_channel_get(
                                                                                            db_tx,
                                                                                            db_owner,
                                                                                            &db_channel_id
                                                                                        )?,
                                                                                        "Channel not found on this server"
                                                                                    )?;
                                                                                check_read_access(
                                                                                    db_tx,
                                                                                    account,
                                                                                    db_owner,
                                                                                    &db_channel_id,
                                                                                    &local,
                                                                                    caller
                                                                                )?;
                                                                                return Ok(
                                                                                    Cached::no_store(
                                                                                        channel_info_res(
                                                                                            db_tx,
                                                                                            account,
                                                                                            db_owner,
                                                                                            &db_channel_id,
                                                                                            &local
                                                                                        )?
                                                                                    )
                                                                                );
                                                                            }
                                                                        ).await;
                                                                    }.await;
                                                                ret
                                                            });
                                                        },
                                                        GET_PREFIX_ACTIVITY => {
                                                            let req =
                                                                match <GetActivity as c2s
                                                                ::PathReqTrait>::deserialize_path(
                                                                    rest
                                                                ) {
                                                                    Ok(r) => r,
                                                                    Err(e) => return s2s_error_response(
                                                                        state,
                                                                        S2sError::BadRequest(e)
                                                                    ),
                                                                };
                                                            return s2s_respond_get::<GetActivity>(state, {
                                                                use {
                                                                    crate::{
                                                                        db,
                                                                        interface::{
                                                                            db::{
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            s2s::{
                                                                                GetActivity,
                                                                                PulledMessage,
                                                                                PulledMessages,
                                                                            },
                                                                        },
                                                                        subsystems::s2s_error::{
                                                                            not_found,
                                                                            s2s_tx,
                                                                            CachePolicy,
                                                                            Cached,
                                                                            S2sError,
                                                                        },
                                                                        State,
                                                                    },
                                                                    loga::ResultContext,
                                                                    std::sync::Arc,
                                                                };
                                                                use crate::subsystems::messages::*;

                                                                let ret: Result<Cached<PulledMessages>, S2sError> =
                                                                    async {
                                                                        let (state, token, req,):
                                                                            (&Arc<State>, &str, GetActivity,) =
                                                                            (state, &auth_token, req,);
                                                                        let caller =
                                                                            resolve_caller(state, token).await?;
                                                                        let db_owner =
                                                                            DbIdentity(req.channel.identity);
                                                                        let db_channel_id =
                                                                            DbChannelId(req.channel.channel);
                                                                        let start = u64_to_i64(req.start);
                                                                        let limit = req.limit.min(PULL_BATCH_LIMIT);
                                                                        return s2s_tx(
                                                                            &state.db,
                                                                            "querying activity",
                                                                            move |db_tx| {
                                                                                let (account, local) =
                                                                                    not_found(
                                                                                        owner_channel_get(
                                                                                            db_tx,
                                                                                            db_owner,
                                                                                            &db_channel_id
                                                                                        )?,
                                                                                        "Channel not found on this server"
                                                                                    )?;
                                                                                check_read_access(
                                                                                    db_tx,
                                                                                    account,
                                                                                    db_owner,
                                                                                    &db_channel_id,
                                                                                    &local,
                                                                                    caller
                                                                                )?;
                                                                                let rows =
                                                                                    good_ormning
                                                                                    ::sqlite
                                                                                    ::good_query_many!(
                                                                                        db,
                                                                                        r#"select seq, snap_offset, receive_time, raw from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and seq >= ${i64 = start} order by seq limit ${i64 = limit as i64}"#;
                                                                                        db_tx
                                                                                    ).map_err(|e| loga::err(e.0))?;
                                                                                let mut messages =
                                                                                    Vec::with_capacity(rows.len());
                                                                                for r in rows {
                                                                                    messages.push(PulledMessage {
                                                                                        seq: i64_to_u64(r.seq),
                                                                                        snap_offset: r
                                                                                            .snap_offset
                                                                                            .map(i64_to_u64),
                                                                                        receive_time: r.receive_time.0,
                                                                                        message: serde_json::from_str(
                                                                                            &r.raw
                                                                                        ).context(
                                                                                            "Stored message was not valid JSON"
                                                                                        )?,
                                                                                    });
                                                                                }
                                                                                let full =
                                                                                    messages.len() as u32 == limit &&
                                                                                        limit > 0;
                                                                                return Ok(Cached {
                                                                                    value: PulledMessages {
                                                                                        messages: messages
                                                                                    },
                                                                                    policy: CachePolicy
                                                                                    ::Immutable.only_if(
                                                                                        local.public_read && full
                                                                                    ),
                                                                                });
                                                                            }
                                                                        ).await;
                                                                    }.await;
                                                                ret
                                                            });
                                                        },
                                                        GET_PREFIX_SNAP_PAGE => {
                                                            let req =
                                                                match <GetSnapPage as c2s
                                                                ::PathReqTrait>::deserialize_path(
                                                                    rest
                                                                ) {
                                                                    Ok(r) => r,
                                                                    Err(e) => return s2s_error_response(
                                                                        state,
                                                                        S2sError::BadRequest(e)
                                                                    ),
                                                                };
                                                            return s2s_respond_get::<GetSnapPage>(state, {
                                                                use {
                                                                    crate::{
                                                                        interface::{
                                                                            db::{
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            s2s::{
                                                                                GetSnapPage,
                                                                                PulledMessages,
                                                                            },
                                                                        },
                                                                        subsystems::s2s_error::{
                                                                            not_found,
                                                                            s2s_tx,
                                                                            CachePolicy,
                                                                            Cached,
                                                                            S2sError,
                                                                            SNAP_PAGE_MAX_AGE_SECS,
                                                                        },
                                                                        State,
                                                                    },
                                                                    std::sync::Arc,
                                                                };
                                                                use crate::subsystems::messages::*;

                                                                let ret:
                                                                    Result<Cached<Option<PulledMessages>>, S2sError> =
                                                                    async {
                                                                        let (state, token, req,):
                                                                            (&Arc<State>, &str, GetSnapPage,) =
                                                                            (state, &auth_token, req,);
                                                                        let caller =
                                                                            resolve_caller(state, token).await?;
                                                                        let db_owner =
                                                                            DbIdentity(req.channel.identity);
                                                                        let db_channel_id =
                                                                            DbChannelId(req.channel.channel);
                                                                        let page = req.page;
                                                                        return s2s_tx(
                                                                            &state.db,
                                                                            "reading snap page",
                                                                            move |db_tx| {
                                                                                let (account, local) =
                                                                                    not_found(
                                                                                        owner_channel_get(
                                                                                            db_tx,
                                                                                            db_owner,
                                                                                            &db_channel_id
                                                                                        )?,
                                                                                        "Channel not found on this server"
                                                                                    )?;
                                                                                check_read_access(
                                                                                    db_tx,
                                                                                    account,
                                                                                    db_owner,
                                                                                    &db_channel_id,
                                                                                    &local,
                                                                                    caller
                                                                                )?;
                                                                                let (originals, edits) =
                                                                                    snap_page_rows(
                                                                                        db_tx,
                                                                                        account,
                                                                                        db_owner,
                                                                                        &db_channel_id,
                                                                                        local.page_size,
                                                                                        page
                                                                                    )?;
                                                                                if originals.is_empty() && page > 0 {
                                                                                    return Ok(Cached::no_store(None));
                                                                                }
                                                                                let mut messages =
                                                                                    Vec::with_capacity(
                                                                                        originals.len() + edits.len()
                                                                                    );
                                                                                for r in originals
                                                                                    .iter()
                                                                                    .chain(edits.iter()) {
                                                                                    messages.push(r.to_pulled()?);
                                                                                }
                                                                                return Ok(Cached {
                                                                                    value: Some(
                                                                                        PulledMessages {
                                                                                            messages: messages
                                                                                        }
                                                                                    ),
                                                                                    policy: CachePolicy::MaxAge(
                                                                                        SNAP_PAGE_MAX_AGE_SECS
                                                                                    ).only_if(local.public_read),
                                                                                });
                                                                            }
                                                                        ).await;
                                                                    }.await;
                                                                ret
                                                            });
                                                        },
                                                        GET_PREFIX_CALENDAR_PAGE => {
                                                            let req =
                                                                match <GetCalendarPage as c2s
                                                                ::PathReqTrait>::deserialize_path(
                                                                    rest
                                                                ) {
                                                                    Ok(r) => r,
                                                                    Err(e) => return s2s_error_response(
                                                                        state,
                                                                        S2sError::BadRequest(e)
                                                                    ),
                                                                };
                                                            return s2s_respond_get::<GetCalendarPage>(state, {
                                                                use {
                                                                    crate::{
                                                                        interface::{
                                                                            db::{
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            s2s::{
                                                                                CalendarPulledPage,
                                                                                CalendarPulledRes,
                                                                                GetCalendarPage,
                                                                            },
                                                                        },
                                                                        subsystems::{
                                                                            calendar::{
                                                                                calendar_local_page,
                                                                                CalendarLocalPage,
                                                                            },
                                                                            s2s_error::{
                                                                                not_found,
                                                                                s2s_tx,
                                                                                CachePolicy,
                                                                                Cached,
                                                                                S2sError,
                                                                                SNAP_PAGE_MAX_AGE_SECS,
                                                                            },
                                                                        },
                                                                        State,
                                                                    },
                                                                    std::sync::Arc,
                                                                };
                                                                use crate::subsystems::messages::*;

                                                                let ret:
                                                                    Result<
                                                                        Cached<Option<CalendarPulledRes>>,
                                                                        S2sError
                                                                    > =
                                                                    async {
                                                                        let (state, token, req,):
                                                                            (&Arc<State>, &str, GetCalendarPage,) =
                                                                            (state, &auth_token, req,);
                                                                        let caller =
                                                                            resolve_caller(state, token).await?;
                                                                        let db_owner =
                                                                            DbIdentity(req.channel.identity);
                                                                        let db_channel_id =
                                                                            DbChannelId(req.channel.channel.clone());
                                                                        let (account, local) =
                                                                            s2s_tx(
                                                                                &state.db,
                                                                                "checking calendar access",
                                                                                move |db_tx| {
                                                                                    let (account, local) =
                                                                                        not_found(
                                                                                            owner_channel_get(
                                                                                                db_tx,
                                                                                                db_owner,
                                                                                                &db_channel_id
                                                                                            )?,
                                                                                            "Channel not found on this server"
                                                                                        )?;
                                                                                    check_read_access(
                                                                                        db_tx,
                                                                                        account,
                                                                                        db_owner,
                                                                                        &db_channel_id,
                                                                                        &local,
                                                                                        caller
                                                                                    )?;
                                                                                    return Ok((account, local));
                                                                                }
                                                                            ).await?;
                                                                        let page =
                                                                            calendar_local_page(
                                                                                state,
                                                                                account,
                                                                                &req.channel,
                                                                                local.page_size,
                                                                                req.month,
                                                                                req.page,
                                                                                req.token
                                                                            ).await?;
                                                                        let value = match page {
                                                                            CalendarLocalPage
                                                                            ::Stale => CalendarPulledRes
                                                                            ::Stale,
                                                                            CalendarLocalPage::Page {
                                                                                token,
                                                                                latest_seq,
                                                                                roots,
                                                                                last
                                                                            } => {
                                                                                let mut messages = vec![];
                                                                                for r in roots {
                                                                                    messages.push(
                                                                                        r.original.to_pulled()?
                                                                                    );
                                                                                    if let Some(e) = r.edit {
                                                                                        messages.push(e.to_pulled()?);
                                                                                    }
                                                                                }
                                                                                CalendarPulledRes::Page(
                                                                                    CalendarPulledPage {
                                                                                        token: token,
                                                                                        latest_seq: latest_seq,
                                                                                        messages: messages,
                                                                                        last: last,
                                                                                    }
                                                                                )
                                                                            },
                                                                        };
                                                                        return Ok(Cached {
                                                                            value: Some(value),
                                                                            policy: CachePolicy::MaxAge(
                                                                                SNAP_PAGE_MAX_AGE_SECS
                                                                            ).only_if(local.public_read),
                                                                        });
                                                                    }.await;
                                                                ret
                                                            });
                                                        },
                                                        GET_PREFIX_SNAP_PAGE_CONTAINING_TIME => {
                                                            let req =
                                                                match <GetSnapPageContainingTime as c2s
                                                                ::PathReqTrait>::deserialize_path(
                                                                    rest
                                                                ) {
                                                                    Ok(r) => r,
                                                                    Err(e) => return s2s_error_response(
                                                                        state,
                                                                        S2sError::BadRequest(e)
                                                                    ),
                                                                };
                                                            return s2s_respond_get::<GetSnapPageContainingTime>(state, {
                                                                use {
                                                                    crate::{
                                                                        interface::{
                                                                            db::{
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            s2s::GetSnapPageContainingTime,
                                                                        },
                                                                        subsystems::s2s_error::{
                                                                            not_found,
                                                                            s2s_tx,
                                                                            Cached,
                                                                            S2sError,
                                                                        },
                                                                        State,
                                                                    },
                                                                    std::sync::Arc,
                                                                };
                                                                use crate::subsystems::messages::*;

                                                                let ret: Result<Cached<Option<u64>>, S2sError> = async {
                                                                    let (state, token, req,):
                                                                        (
                                                                            &Arc<State>,
                                                                            &str,
                                                                            GetSnapPageContainingTime,
                                                                        ) =
                                                                        (state, &auth_token, req,);
                                                                    let caller = resolve_caller(state, token).await?;
                                                                    let db_owner = DbIdentity(req.channel.identity);
                                                                    let db_channel_id =
                                                                        DbChannelId(req.channel.channel);
                                                                    let time = req.time;
                                                                    return s2s_tx(
                                                                        &state.db,
                                                                        "finding snap page for time",
                                                                        move |db_tx| {
                                                                            let (account, local) =
                                                                                not_found(
                                                                                    owner_channel_get(
                                                                                        db_tx,
                                                                                        db_owner,
                                                                                        &db_channel_id
                                                                                    )?,
                                                                                    "Channel not found on this server"
                                                                                )?;
                                                                            check_read_access(
                                                                                db_tx,
                                                                                account,
                                                                                db_owner,
                                                                                &db_channel_id,
                                                                                &local,
                                                                                caller
                                                                            )?;
                                                                            let Some(offset) =
                                                                                snap_offset_containing_time(
                                                                                    db_tx,
                                                                                    account,
                                                                                    db_owner,
                                                                                    &db_channel_id,
                                                                                    time
                                                                                )? else {
                                                                                    return Ok(Cached::no_store(None));
                                                                                };
                                                                            return Ok(
                                                                                Cached::no_store(
                                                                                    Some(
                                                                                        i64_to_u64(offset) /
                                                                                            local.page_size as u64
                                                                                    )
                                                                                )
                                                                            );
                                                                        }
                                                                    ).await;
                                                                }.await;
                                                                ret
                                                            });
                                                        },
                                                        GET_PREFIX_SNAP_LOOKUP => {
                                                            let req =
                                                                match <GetSnapLookup as c2s
                                                                ::PathReqTrait>::deserialize_path(
                                                                    rest
                                                                ) {
                                                                    Ok(r) => r,
                                                                    Err(e) => return s2s_error_response(
                                                                        state,
                                                                        S2sError::BadRequest(e)
                                                                    ),
                                                                };
                                                            return s2s_respond_get::<GetSnapLookup>(state, {
                                                                use {
                                                                    crate::{
                                                                        interface::{
                                                                            db::{
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            s2s::{
                                                                                GetSnapLookup,
                                                                                SnapLookupBy,
                                                                                SnapLookupRes,
                                                                            },
                                                                        },
                                                                        subsystems::s2s_error::{
                                                                            not_found,
                                                                            s2s_tx,
                                                                            CachePolicy,
                                                                            Cached,
                                                                            S2sError,
                                                                        },
                                                                        State,
                                                                    },
                                                                    std::sync::Arc,
                                                                };
                                                                use crate::subsystems::messages::*;

                                                                let ret:
                                                                    Result<Cached<Option<SnapLookupRes>>, S2sError> =
                                                                    async {
                                                                        let (state, token, req,):
                                                                            (&Arc<State>, &str, GetSnapLookup,) =
                                                                            (state, &auth_token, req,);
                                                                        let caller =
                                                                            resolve_caller(state, token).await?;
                                                                        let db_owner =
                                                                            DbIdentity(req.channel.identity);
                                                                        let db_channel_id =
                                                                            DbChannelId(req.channel.channel);
                                                                        let by = req.by;
                                                                        return s2s_tx(
                                                                            &state.db,
                                                                            "looking up snap position",
                                                                            move |db_tx| {
                                                                                let (account, local) =
                                                                                    not_found(
                                                                                        owner_channel_get(
                                                                                            db_tx,
                                                                                            db_owner,
                                                                                            &db_channel_id
                                                                                        )?,
                                                                                        "Channel not found on this server"
                                                                                    )?;
                                                                                check_read_access(
                                                                                    db_tx,
                                                                                    account,
                                                                                    db_owner,
                                                                                    &db_channel_id,
                                                                                    &local,
                                                                                    caller
                                                                                )?;
                                                                                let pos = match by {
                                                                                    SnapLookupBy::Id(
                                                                                        id
                                                                                    ) => snap_position_by_id(
                                                                                        db_tx,
                                                                                        account,
                                                                                        db_owner,
                                                                                        &db_channel_id,
                                                                                        &id
                                                                                    )?,
                                                                                    SnapLookupBy::ClientId(
                                                                                        sender,
                                                                                        client_id
                                                                                    ) => snap_position_by_client_id(
                                                                                        db_tx,
                                                                                        account,
                                                                                        db_owner,
                                                                                        &db_channel_id,
                                                                                        sender,
                                                                                        &client_id.0,
                                                                                    )?,
                                                                                };
                                                                                let found = pos.is_some();
                                                                                return Ok(Cached {
                                                                                    value: pos.map(|p| SnapLookupRes {
                                                                                        original_id: p.original_id,
                                                                                        snap_offset: i64_to_u64(
                                                                                            p.snap_offset
                                                                                        ),
                                                                                        receive_time: p.receive_time,
                                                                                    }),
                                                                                    policy: CachePolicy
                                                                                    ::Immutable.only_if(
                                                                                        local.public_read && found
                                                                                    ),
                                                                                });
                                                                            }
                                                                        ).await;
                                                                    }.await;
                                                                ret
                                                            });
                                                        },
                                                        GET_PREFIX_MESSAGE_HISTORY => {
                                                            let req =
                                                                match <GetMessageHistory as c2s
                                                                ::PathReqTrait>::deserialize_path(
                                                                    rest
                                                                ) {
                                                                    Ok(r) => r,
                                                                    Err(e) => return s2s_error_response(
                                                                        state,
                                                                        S2sError::BadRequest(e)
                                                                    ),
                                                                };
                                                            return s2s_respond_get::<GetMessageHistory>(state, {
                                                                use {
                                                                    crate::{
                                                                        interface::{
                                                                            db::{
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            s2s::{
                                                                                GetMessageHistory,
                                                                                PulledMessages,
                                                                            },
                                                                        },
                                                                        subsystems::s2s_error::{
                                                                            not_found,
                                                                            s2s_tx,
                                                                            Cached,
                                                                            S2sError,
                                                                        },
                                                                        State,
                                                                    },
                                                                    std::sync::Arc,
                                                                };
                                                                use crate::subsystems::messages::*;

                                                                let ret:
                                                                    Result<Cached<Option<PulledMessages>>, S2sError> =
                                                                    async {
                                                                        let (state, token, req,):
                                                                            (&Arc<State>, &str, GetMessageHistory,) =
                                                                            (state, &auth_token, req,);
                                                                        let caller =
                                                                            resolve_caller(state, token).await?;
                                                                        let db_owner =
                                                                            DbIdentity(req.channel.identity);
                                                                        let db_channel_id =
                                                                            DbChannelId(req.channel.channel);
                                                                        let id = req.id;
                                                                        return s2s_tx(
                                                                            &state.db,
                                                                            "reading message history",
                                                                            move |db_tx| {
                                                                                let (account, local) =
                                                                                    not_found(
                                                                                        owner_channel_get(
                                                                                            db_tx,
                                                                                            db_owner,
                                                                                            &db_channel_id
                                                                                        )?,
                                                                                        "Channel not found on this server"
                                                                                    )?;
                                                                                check_read_access(
                                                                                    db_tx,
                                                                                    account,
                                                                                    db_owner,
                                                                                    &db_channel_id,
                                                                                    &local,
                                                                                    caller
                                                                                )?;
                                                                                let Some(rows) =
                                                                                    message_history_rows(
                                                                                        db_tx,
                                                                                        account,
                                                                                        db_owner,
                                                                                        &db_channel_id,
                                                                                        &id
                                                                                    )? else {
                                                                                        return Ok(
                                                                                            Cached::no_store(None)
                                                                                        );
                                                                                    };
                                                                                let mut messages =
                                                                                    Vec::with_capacity(rows.len());
                                                                                for r in &rows {
                                                                                    messages.push(r.to_pulled()?);
                                                                                }
                                                                                return Ok(
                                                                                    Cached::no_store(
                                                                                        Some(
                                                                                            PulledMessages {
                                                                                                messages: messages
                                                                                            }
                                                                                        )
                                                                                    )
                                                                                );
                                                                            }
                                                                        ).await;
                                                                    }.await;
                                                                ret
                                                            });
                                                        },
                                                        GET_PREFIX_JOIN_PREVIEW => {
                                                            let req =
                                                                match <GetJoinPreview as c2s
                                                                ::PathReqTrait>::deserialize_path(
                                                                    rest
                                                                ) {
                                                                    Ok(r) => r,
                                                                    Err(e) => return s2s_error_response(
                                                                        state,
                                                                        S2sError::BadRequest(e)
                                                                    ),
                                                                };
                                                            return s2s_respond_get::<GetJoinPreview>(state, {
                                                                use {
                                                                    crate::{
                                                                        interface::{
                                                                            db::{
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            s2s::{
                                                                                GetJoinPreview,
                                                                                JoinPreviewRes,
                                                                                JoinPreviewTarget,
                                                                            },
                                                                        },
                                                                        subsystems::s2s_error::{
                                                                            not_found,
                                                                            s2s_tx,
                                                                            Cached,
                                                                            S2sError,
                                                                        },
                                                                        State,
                                                                    },
                                                                    std::sync::Arc,
                                                                };
                                                                use crate::subsystems::messages::*;

                                                                let ret: Result<Cached<JoinPreviewRes>, S2sError> =
                                                                    async {
                                                                        let (state, token, req,):
                                                                            (&Arc<State>, &str, GetJoinPreview,) =
                                                                            (state, &auth_token, req,);
                                                                        let caller =
                                                                            resolve_caller(state, token).await?;
                                                                        return s2s_tx(
                                                                            &state.db,
                                                                            "reading join preview",
                                                                            move |db_tx| {
                                                                                match req.target {
                                                                                    JoinPreviewTarget::IdentityInvite {
                                                                                        identity,
                                                                                        code
                                                                                    } => {
                                                                                        let db_identity =
                                                                                            DbIdentity(identity);
                                                                                        let invite =
                                                                                            lookup_identity_invite(
                                                                                                db_tx,
                                                                                                db_identity,
                                                                                                &code
                                                                                            )?;
                                                                                        let meta =
                                                                                            owner_identity_meta(
                                                                                                db_tx,
                                                                                                invite.account_id,
                                                                                                db_identity
                                                                                            )?;
                                                                                        return Ok(
                                                                                            Cached::no_store(
                                                                                                JoinPreviewRes {
                                                                                                    name: invite
                                                                                                        .shown_name
                                                                                                        .or(
                                                                                                            meta.shown_name
                                                                                                        )
                                                                                                        .unwrap_or(
                                                                                                            meta.name
                                                                                                        ),
                                                                                                    description: invite
                                                                                                        .shown_description
                                                                                                        .or(
                                                                                                            meta.shown_description
                                                                                                        )
                                                                                                        .unwrap_or(
                                                                                                            meta.description
                                                                                                        ),
                                                                                                    portrait: crate
                                                                                                    ::subsystems
                                                                                                    ::files
                                                                                                    ::file_ref(
                                                                                                        db_tx,
                                                                                                        invite.account_id,
                                                                                                        meta.portrait
                                                                                                    )?,
                                                                                                }
                                                                                            )
                                                                                        );
                                                                                    },
                                                                                    JoinPreviewTarget::ChannelInvite {
                                                                                        channel,
                                                                                        code
                                                                                    } => {
                                                                                        let db_owner =
                                                                                            DbIdentity(
                                                                                                channel.identity
                                                                                            );
                                                                                        let db_channel_id =
                                                                                            DbChannelId(
                                                                                                channel.channel
                                                                                            );
                                                                                        let invite =
                                                                                            lookup_channel_invite(
                                                                                                db_tx,
                                                                                                &db_channel_id,
                                                                                                &code
                                                                                            )?;
                                                                                        if invite.owner.0 !=
                                                                                            channel.identity {
                                                                                            return Err(
                                                                                                S2sError::NotFound(
                                                                                                    "Invitation not recognized".to_string()
                                                                                                )
                                                                                            );
                                                                                        }
                                                                                        let meta =
                                                                                            owner_channel_meta(
                                                                                                db_tx,
                                                                                                invite.account_id,
                                                                                                db_owner,
                                                                                                &db_channel_id
                                                                                            )?;
                                                                                        return Ok(
                                                                                            Cached::no_store(
                                                                                                JoinPreviewRes {
                                                                                                    name: invite
                                                                                                        .shown_name
                                                                                                        .or(
                                                                                                            meta.shown_name
                                                                                                        )
                                                                                                        .unwrap_or(
                                                                                                            meta.name
                                                                                                        ),
                                                                                                    description: invite
                                                                                                        .shown_description
                                                                                                        .or(
                                                                                                            meta.shown_description
                                                                                                        )
                                                                                                        .unwrap_or(
                                                                                                            meta.description
                                                                                                        ),
                                                                                                    portrait: crate
                                                                                                    ::subsystems
                                                                                                    ::files
                                                                                                    ::file_ref(
                                                                                                        db_tx,
                                                                                                        invite.account_id,
                                                                                                        meta.portrait
                                                                                                    )?,
                                                                                                }
                                                                                            )
                                                                                        );
                                                                                    },
                                                                                    JoinPreviewTarget::Channel(
                                                                                        channel
                                                                                    ) => {
                                                                                        let db_owner =
                                                                                            DbIdentity(
                                                                                                channel.identity
                                                                                            );
                                                                                        let db_channel_id =
                                                                                            DbChannelId(
                                                                                                channel.channel
                                                                                            );
                                                                                        let (account, local) =
                                                                                            not_found(
                                                                                                owner_channel_get(
                                                                                                    db_tx,
                                                                                                    db_owner,
                                                                                                    &db_channel_id
                                                                                                )?,
                                                                                                "Channel not found on this server",
                                                                                            )?;
                                                                                        check_read_access(
                                                                                            db_tx,
                                                                                            account,
                                                                                            db_owner,
                                                                                            &db_channel_id,
                                                                                            &local,
                                                                                            caller
                                                                                        )?;
                                                                                        let meta =
                                                                                            owner_channel_meta(
                                                                                                db_tx,
                                                                                                account,
                                                                                                db_owner,
                                                                                                &db_channel_id
                                                                                            )?;
                                                                                        return Ok(
                                                                                            Cached::no_store(
                                                                                                JoinPreviewRes {
                                                                                                    name: meta
                                                                                                        .shown_name
                                                                                                        .unwrap_or(
                                                                                                            meta.name
                                                                                                        ),
                                                                                                    description: meta
                                                                                                        .shown_description
                                                                                                        .unwrap_or(
                                                                                                            meta.description
                                                                                                        ),
                                                                                                    portrait: crate
                                                                                                    ::subsystems
                                                                                                    ::files
                                                                                                    ::file_ref(
                                                                                                        db_tx,
                                                                                                        account,
                                                                                                        meta.portrait
                                                                                                    )?,
                                                                                                }
                                                                                            )
                                                                                        );
                                                                                    },
                                                                                }
                                                                            }
                                                                        ).await;
                                                                    }.await;
                                                                ret
                                                            });
                                                        },
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
                                                            match {
                                                                use {
                                                                    crate::{
                                                                        db,
                                                                        interface::{
                                                                            db::{
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            s2s::{
                                                                                FileAuth,
                                                                                GetFile,
                                                                            },
                                                                        },
                                                                        subsystems::s2s_error::{
                                                                            not_found,
                                                                            s2s_tx,
                                                                            S2sError,
                                                                        },
                                                                        State,
                                                                    },
                                                                    std::sync::Arc,
                                                                };
                                                                use crate::subsystems::messages::*;

                                                                let ret:
                                                                    Result<(String, std::path::PathBuf), S2sError> =
                                                                    async {
                                                                        let (state, token, req,):
                                                                            (&Arc<State>, &str, GetFile,) =
                                                                            (state, &auth_token, req,);
                                                                        let hash = req.hash.clone();
                                                                        let account = match req.auth {
                                                                            FileAuth::IdentityInvite {
                                                                                identity,
                                                                                code
                                                                            } => {
                                                                                let db_identity =
                                                                                    DbIdentity(identity);
                                                                                s2s_tx(
                                                                                    &state.db,
                                                                                    "checking invited identity file access",
                                                                                    move |db_tx| {
                                                                                        let invite =
                                                                                            lookup_identity_invite(
                                                                                                db_tx,
                                                                                                db_identity,
                                                                                                &code
                                                                                            )?;
                                                                                        let meta =
                                                                                            owner_identity_meta(
                                                                                                db_tx,
                                                                                                invite.account_id,
                                                                                                db_identity
                                                                                            )?;
                                                                                        if meta.portrait.as_deref() !=
                                                                                            Some(hash.0.as_str()) {
                                                                                            return Err(
                                                                                                S2sError::Forbidden(
                                                                                                    "File is not part of this invitation".to_string()
                                                                                                )
                                                                                            );
                                                                                        }
                                                                                        return Ok(invite.account_id);
                                                                                    }
                                                                                ).await?
                                                                            },
                                                                            FileAuth::ChannelInvite {
                                                                                channel,
                                                                                code
                                                                            } => {
                                                                                let db_owner =
                                                                                    DbIdentity(channel.identity);
                                                                                let db_channel_id =
                                                                                    DbChannelId(channel.channel);
                                                                                s2s_tx(
                                                                                    &state.db,
                                                                                    "checking invited channel file access",
                                                                                    move |db_tx| {
                                                                                        let invite =
                                                                                            lookup_channel_invite(
                                                                                                db_tx,
                                                                                                &db_channel_id,
                                                                                                &code
                                                                                            )?;
                                                                                        if invite.owner.0 !=
                                                                                            channel.identity {
                                                                                            return Err(
                                                                                                S2sError::NotFound(
                                                                                                    "Invitation not recognized".to_string()
                                                                                                )
                                                                                            );
                                                                                        }
                                                                                        let meta =
                                                                                            owner_channel_meta(
                                                                                                db_tx,
                                                                                                invite.account_id,
                                                                                                db_owner,
                                                                                                &db_channel_id
                                                                                            )?;
                                                                                        if meta.portrait.as_deref() !=
                                                                                            Some(hash.0.as_str()) {
                                                                                            return Err(
                                                                                                S2sError::Forbidden(
                                                                                                    "File is not part of this invitation".to_string()
                                                                                                )
                                                                                            );
                                                                                        }
                                                                                        return Ok(invite.account_id);
                                                                                    }
                                                                                ).await?
                                                                            },
                                                                            FileAuth::Channel(channel) => {
                                                                                let caller =
                                                                                    resolve_caller(
                                                                                        state,
                                                                                        token
                                                                                    ).await?;
                                                                                let db_owner =
                                                                                    DbIdentity(channel.identity);
                                                                                let db_channel_id =
                                                                                    DbChannelId(channel.channel);
                                                                                s2s_tx(
                                                                                    &state.db,
                                                                                    "checking channel file access",
                                                                                    move |db_tx| {
                                                                                        let accounts =
                                                                                            good_ormning
                                                                                            ::sqlite
                                                                                            ::good_query_many!(
                                                                                                db,
                                                                                                r#"select distinct account_id from message_file where identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and hash = ${string = hash.0.as_str()}"#;
                                                                                                db_tx
                                                                                            ).map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?;
                                                                                        for account in accounts {
                                                                                            let Some(local) =
                                                                                                local_channel_get(
                                                                                                    db_tx,
                                                                                                    account,
                                                                                                    db_owner,
                                                                                                    &db_channel_id
                                                                                                )? else {
                                                                                                    continue;
                                                                                                };
                                                                                            if caller ==
                                                                                                Some(db_owner.0) {
                                                                                                return Ok(account);
                                                                                            }
                                                                                            if local.is_owner(
                                                                                                db_owner.0
                                                                                            ) &&
                                                                                                check_read_access(
                                                                                                    db_tx,
                                                                                                    account,
                                                                                                    db_owner,
                                                                                                    &db_channel_id,
                                                                                                    &local,
                                                                                                    caller
                                                                                                ).is_ok() {
                                                                                                return Ok(account);
                                                                                            }
                                                                                        }
                                                                                        if caller.is_some() {
                                                                                            return Err(
                                                                                                S2sError::Forbidden(
                                                                                                    "Caller has no access to this file".to_string()
                                                                                                )
                                                                                            );
                                                                                        } else {
                                                                                            return Err(
                                                                                                S2sError::Unauthorized(
                                                                                                    "File requires authentication".to_string()
                                                                                                )
                                                                                            );
                                                                                        }
                                                                                    }
                                                                                ).await?
                                                                            },
                                                                            FileAuth::Caller => {
                                                                                let caller =
                                                                                    resolve_caller(
                                                                                        state,
                                                                                        token
                                                                                    ).await?;
                                                                                s2s_tx(
                                                                                    &state.db,
                                                                                    "checking file access",
                                                                                    move |db_tx| {
                                                                                        let identities =
                                                                                            good_ormning
                                                                                            ::sqlite
                                                                                            ::good_query_many!(
                                                                                                db,
                                                                                                r#"select account_id, id from identity where portrait = ${string = hash.0.as_str()} and soft_deleted_at is null"#;
                                                                                                db_tx
                                                                                            ).map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?;
                                                                                        if let Some(caller) = caller {
                                                                                            let db_caller =
                                                                                                DbIdentity(caller);
                                                                                            for i in identities {
                                                                                                let identity_owns =
                                                                                                    good_ormning
                                                                                                    ::sqlite
                                                                                                    ::good_query_opt!(
                                                                                                        db,
                                                                                                        r#"select 1 as x from channel_member where account_id = ${account_id_t = &i.account_id} and identity = ${identity_id_t = &i.id} and member = ${identity_id_t = &db_caller}"#;
                                                                                                        db_tx
                                                                                                    ).map_err(
                                                                                                        |e| loga::err(
                                                                                                            e.0
                                                                                                        )
                                                                                                    )?;
                                                                                                if identity_owns.is_some() {
                                                                                                    return Ok(
                                                                                                        i.account_id
                                                                                                    );
                                                                                                }
                                                                                                let caller_owns =
                                                                                                    good_ormning
                                                                                                    ::sqlite
                                                                                                    ::good_query_opt!(
                                                                                                        db,
                                                                                                        r#"select 1 as x from channel where account_id = ${account_id_t = &i.account_id} and identity = ${identity_id_t = &db_caller} and local_identity = ${identity_id_t = &i.id} and deleted is null"#;
                                                                                                        db_tx
                                                                                                    ).map_err(
                                                                                                        |e| loga::err(
                                                                                                            e.0
                                                                                                        )
                                                                                                    )?;
                                                                                                if caller_owns.is_some() {
                                                                                                    return Ok(
                                                                                                        i.account_id
                                                                                                    );
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                        let channels =
                                                                                            good_ormning
                                                                                            ::sqlite
                                                                                            ::good_query_many!(
                                                                                                db,
                                                                                                r#"select account_id, identity, id, local_identity from channel where portrait = ${string = hash.0.as_str()} and deleted is null"#;
                                                                                                db_tx
                                                                                            ).map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?;
                                                                                        for c in channels {
                                                                                            if c.identity.0 !=
                                                                                                c.local_identity.0 {
                                                                                                continue;
                                                                                            }
                                                                                            let Some(local) =
                                                                                                local_channel_get(
                                                                                                    db_tx,
                                                                                                    c.account_id,
                                                                                                    c.identity,
                                                                                                    &c.id
                                                                                                )? else {
                                                                                                    continue;
                                                                                                };
                                                                                            if check_read_access(
                                                                                                db_tx,
                                                                                                c.account_id,
                                                                                                c.identity,
                                                                                                &c.id,
                                                                                                &local,
                                                                                                caller
                                                                                            ).is_ok() {
                                                                                                return Ok(
                                                                                                    c.account_id
                                                                                                );
                                                                                            }
                                                                                        }
                                                                                        if caller.is_some() {
                                                                                            return Err(
                                                                                                S2sError::Forbidden(
                                                                                                    "Caller has no access to this file".to_string()
                                                                                                )
                                                                                            );
                                                                                        } else {
                                                                                            return Err(
                                                                                                S2sError::Unauthorized(
                                                                                                    "File requires authentication".to_string()
                                                                                                )
                                                                                            );
                                                                                        }
                                                                                    }
                                                                                ).await?
                                                                            },
                                                                        };
                                                                        return Ok(
                                                                            not_found(
                                                                                crate::subsystems::files::file_local(
                                                                                    state,
                                                                                    account,
                                                                                    &req.hash,
                                                                                    req.thumbnail
                                                                                ).await?,
                                                                                "File not found"
                                                                            )?
                                                                        );
                                                                    }.await;
                                                                ret
                                                            } {
                                                                Ok((mime, path)) => {
                                                                    let mut file_headers = HeaderMap::new();
                                                                    file_headers.insert(
                                                                        X_CONTENT_TYPE_OPTIONS,
                                                                        http::HeaderValue::from_static("nosniff")
                                                                    );
                                                                    match htwrap::htserve::responses::response_file(
                                                                        &head.headers,
                                                                        &mime,
                                                                        &path,
                                                                        true,
                                                                        &file_headers
                                                                    ).await {
                                                                        Ok(r) => return r,
                                                                        Err(e) => return s2s_error_response(
                                                                            state,
                                                                            S2sError::Internal(
                                                                                loga::err(
                                                                                    e.to_string()
                                                                                ).context("Error serving file")
                                                                            )
                                                                        ),
                                                                    }
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
                                                    {
                                                        use {
                                                            crate::State,
                                                            futures::{
                                                                SinkExt,
                                                                StreamExt,
                                                            },
                                                            hyper::upgrade::Upgraded,
                                                            hyper_tungstenite::{
                                                                tungstenite::Message as WsMessage,
                                                                WebSocketStream,
                                                            },
                                                            hyper_util::rt::TokioIo,
                                                            shared::interface::shared::AccountId,
                                                            std::sync::Arc,
                                                        };

                                                        async {
                                                            let (state, account_id, ws,):
                                                                (
                                                                    &Arc<State>,
                                                                    AccountId,
                                                                    WebSocketStream<TokioIo<Upgraded>>,
                                                                ) =
                                                                (&state, account, ws,);
                                                            let mut sub = {
                                                                use {
                                                                    shared::interface::shared::AccountId,
                                                                    std::{
                                                                        collections::HashMap,
                                                                        sync::atomic::Ordering,
                                                                    },
                                                                    tokio::sync::mpsc,
                                                                };
                                                                use crate::subsystems::ws_bus::*;

                                                                let ret: Subscription = async {
                                                                    let (ws_bus, account_id,): (&WsBus, AccountId,) =
                                                                        (&(state.ws_bus), account_id,);
                                                                    let id =
                                                                        ws_bus
                                                                            .next_id
                                                                            .fetch_add(1, Ordering::Relaxed);
                                                                    let (tx, rx) = mpsc::channel(256);
                                                                    ws_bus
                                                                        .subscribers
                                                                        .write()
                                                                        .await
                                                                        .entry(account_id)
                                                                        .or_insert_with(HashMap::new)
                                                                        .insert(id, tx);
                                                                    return Subscription {
                                                                        account_id: account_id,
                                                                        id: id,
                                                                        receiver: rx,
                                                                    };
                                                                }.await;
                                                                ret
                                                            };
                                                            let (mut sink, mut stream) = ws.split();
                                                            let receiver = &mut sub.receiver;
                                                            let write_task = async {
                                                                while let Some(n) = receiver.recv().await {
                                                                    let payload = match serde_json::to_string(&n) {
                                                                        Ok(p) => p,
                                                                        Err(e) => {
                                                                            state
                                                                                .log
                                                                                .log_err(
                                                                                    loga::WARN,
                                                                                    loga::err(
                                                                                        e.to_string()
                                                                                    ).context(
                                                                                        "Failed to serialize WS notification"
                                                                                    ),
                                                                                );
                                                                            continue;
                                                                        },
                                                                    };
                                                                    if sink
                                                                        .send(WsMessage::Text(payload.into()))
                                                                        .await
                                                                        .is_err() {
                                                                        break;
                                                                    }
                                                                }
                                                            };
                                                            let read_task = async {
                                                                while let Some(m) = stream.next().await {
                                                                    match m {
                                                                        Ok(WsMessage::Close(_)) | Err(_) => break,
                                                                        _ => { },
                                                                    }
                                                                }
                                                            };
                                                            tokio::select!{
                                                                _ = write_task => {
                                                                },
                                                                _ = read_task => {
                                                                },
                                                            }
                                                            {
                                                                use crate::subsystems::ws_bus::*;

                                                                async {
                                                                    let (ws_bus, sub,): (&WsBus, &Subscription,) =
                                                                        (&(state.ws_bus), &sub,);
                                                                    let mut subs = ws_bus.subscribers.write().await;
                                                                    if let Some(m) = subs.get_mut(&sub.account_id) {
                                                                        m.remove(&sub.id);
                                                                        if m.is_empty() {
                                                                            subs.remove(&sub.account_id);
                                                                        }
                                                                    }
                                                                }.await
                                                            };
                                                        }.await
                                                    };
                                                });
                                                return Ok(
                                                    response.map(
                                                        |b| b.map_err(|e| std::io::Error::other(e.to_string())).boxed()
                                                    )
                                                );
                                            }
                                            match first {
                                                "oidc" => {
                                                    return Ok({
                                                        use {
                                                            crate::{
                                                                cap_fn,
                                                                db,
                                                                dbutil::tx,
                                                                interface::AccountExternalId,
                                                            },
                                                            cookie::{
                                                                CookieBuilder,
                                                                SameSite,
                                                            },
                                                            deadpool_sqlite::Pool,
                                                            http::{
                                                                request::Parts,
                                                                Response,
                                                            },
                                                            htwrap::htserve::{
                                                                responses::{
                                                                    body_empty,
                                                                    response_400,
                                                                    Body,
                                                                },
                                                                viserr::{
                                                                    ResultVisErr,
                                                                    VisErr,
                                                                },
                                                            },
                                                            loga::{
                                                                ea,
                                                                ResultContext,
                                                            },
                                                            openidconnect::{
                                                                core::CoreAuthenticationFlow,
                                                                AccessTokenHash,
                                                                AuthorizationCode,
                                                                CsrfToken,
                                                                Nonce,
                                                                OAuth2TokenResponse,
                                                                PkceCodeChallenge,
                                                                TokenResponse,
                                                            },
                                                            rand::distr::{
                                                                Alphanumeric,
                                                                SampleString,
                                                            },
                                                            serde::Deserialize,
                                                            std::sync::{
                                                                Arc,
                                                                Mutex,
                                                            },
                                                        };
                                                        use crate::subsystems::oidc::*;

                                                        let ret: Result<Response<Body>, VisErr<loga::Error>> = async {
                                                            let (state, db, head,): (&OidcState, &Pool, Parts,) =
                                                                (&state.oidc_state, &state.db, head,);
                                                            let log = state.log.clone();

                                                            #[derive(Deserialize)]
                                                            struct CallbackParams {
                                                                code: String,
                                                                state: String,
                                                            }

                                                            if let Some(params) =
                                                                head
                                                                    .uri
                                                                    .query()
                                                                    .and_then(
                                                                        |q| serde_urlencoded
                                                                        ::from_str::<CallbackParams>(
                                                                            q
                                                                        ).ok()
                                                                    ) {
                                                                let Some(pre_session_state) =
                                                                    state
                                                                        .pre_sessions
                                                                        .remove(&params.state)
                                                                        .await else {
                                                                        log.log_with(
                                                                            loga::DEBUG,
                                                                            "Missing pre-session state for state",
                                                                            ea!(state = params.state)
                                                                        );
                                                                        return Ok(
                                                                            response_400(
                                                                                "Login attempt expired or unknown"
                                                                            )
                                                                        );
                                                                    };
                                                                let Some(pkce_verifier) =
                                                                    pre_session_state
                                                                        .pkce_verifier
                                                                        .lock()
                                                                        .unwrap()
                                                                        .take() else {
                                                                        return Ok(
                                                                            response_400("Login attempt already used")
                                                                        );
                                                                    };
                                                                let token_response =
                                                                    state
                                                                        .client
                                                                        .exchange_code(
                                                                            AuthorizationCode::new(params.code)
                                                                        )
                                                                        .context("OIDC provider has no token endpoint")
                                                                        .err_internal()?
                                                                        .set_pkce_verifier(pkce_verifier)
                                                                        .request_async(&cap_fn!((r)(log) {
                                                                            return oidc_http_client(&log, r)
                                                                                .await
                                                                                .map_err(
                                                                                    |e| std::io::Error::other(
                                                                                        e.to_string()
                                                                                    )
                                                                                );
                                                                        }))
                                                                        .await
                                                                        .context(
                                                                            "Error exchanging token from callback with OIDC server"
                                                                        )
                                                                        .err_internal()?;
                                                                let id_token =
                                                                    token_response
                                                                        .id_token()
                                                                        .context(
                                                                            "OIDC server response missing ID token"
                                                                        )
                                                                        .err_internal()?;
                                                                let claims =
                                                                    id_token
                                                                        .claims(
                                                                            &state.client.id_token_verifier(),
                                                                            &pre_session_state.nonce
                                                                        )
                                                                        .context(
                                                                            "Error getting claims from OIDC server response"
                                                                        )
                                                                        .err_internal()?;
                                                                if let Some(expected_access_token_hash) =
                                                                    claims.access_token_hash() {
                                                                    let actual_access_token_hash =
                                                                        AccessTokenHash::from_token(
                                                                            token_response.access_token(),
                                                                            id_token
                                                                                .signing_alg()
                                                                                .context(
                                                                                    "Error getting signing alg from server response"
                                                                                )
                                                                                .err_internal()?,
                                                                            id_token
                                                                                .signing_key(
                                                                                    &state.client.id_token_verifier()
                                                                                )
                                                                                .context(
                                                                                    "Error getting signing key from server response"
                                                                                )
                                                                                .err_internal()?,
                                                                        )
                                                                            .context(
                                                                                "Error hashing access token in server response"
                                                                            )
                                                                            .err_internal()?;
                                                                    if actual_access_token_hash !=
                                                                        *expected_access_token_hash {
                                                                        log.log_with(
                                                                            loga::DEBUG,
                                                                            "Access token hash mismatch",
                                                                            ea!(
                                                                                want = *expected_access_token_hash,
                                                                                got = actual_access_token_hash
                                                                            ),
                                                                        );
                                                                        return Ok(
                                                                            response_400("Access token hash mismatch")
                                                                        );
                                                                    }
                                                                }
                                                                let session_cookie =
                                                                    Alphanumeric.sample_string(&mut rand::rng(), 32);
                                                                let now = jiff::Timestamp::now();
                                                                let session = session_cookie.clone();
                                                                let external_id =
                                                                    AccountExternalId(claims.subject().to_string());
                                                                tx(db, move |db_tx| {
                                                                    good_ormning::sqlite::good_query!(
                                                                        db,
                                                                        r#"insert into session (id, external_id, last_used) values (${string = session.as_str()}, ${string = external_id.0.as_str()}, ${time_ms_t = &crate::interface::db::DbTimeMs(now)})"#;
                                                                        db_tx
                                                                    ).map_err(|e| loga::err(e.0))?;
                                                                    return Ok(());
                                                                })
                                                                    .await
                                                                    .context("Error storing session")
                                                                    .err_internal()?;
                                                                return Ok(
                                                                    http::Response::builder()
                                                                        .status(http::StatusCode::TEMPORARY_REDIRECT)
                                                                        .header(
                                                                            http::header::SET_COOKIE,
                                                                            CookieBuilder::new(
                                                                                COOKIE_SESSION,
                                                                                session_cookie
                                                                            )
                                                                                .http_only(true)
                                                                                .secure(true)
                                                                                .same_site(SameSite::Lax)
                                                                                .permanent()
                                                                                .build()
                                                                                .to_string(),
                                                                        )
                                                                        .header(
                                                                            http::header::LOCATION,
                                                                            state.public_url.to_string()
                                                                        )
                                                                        .body(body_empty())
                                                                        .unwrap(),
                                                                );
                                                            }
                                                            let (pkce_challenge, pkce_verifier) =
                                                                PkceCodeChallenge::new_random_sha256();
                                                            let (auth_url, csrf_token, nonce) =
                                                                state
                                                                    .client
                                                                    .authorize_url(
                                                                        CoreAuthenticationFlow::AuthorizationCode,
                                                                        CsrfToken::new_random,
                                                                        Nonce::new_random
                                                                    )
                                                                    .set_pkce_challenge(pkce_challenge)
                                                                    .url();
                                                            state
                                                                .pre_sessions
                                                                .insert(
                                                                    csrf_token.secret().clone(),
                                                                    Arc::new(OidcPreSession {
                                                                        pkce_verifier: Mutex::new(Some(pkce_verifier)),
                                                                        nonce: nonce,
                                                                    })
                                                                )
                                                                .await;
                                                            return Ok(
                                                                http::Response::builder()
                                                                    .status(http::StatusCode::TEMPORARY_REDIRECT)
                                                                    .header(
                                                                        http::header::LOCATION,
                                                                        auth_url.to_string()
                                                                    )
                                                                    .body(body_empty())
                                                                    .unwrap(),
                                                            );
                                                        }.await;
                                                        ret
                                                    }?);
                                                },
                                                "logout" => {
                                                    return Ok({
                                                        use {
                                                            deadpool_sqlite::Pool,
                                                            http::{
                                                                request::Parts,
                                                                Response,
                                                            },
                                                            htwrap::htserve::{
                                                                responses::{
                                                                    body_empty,
                                                                    Body,
                                                                },
                                                                viserr::{
                                                                    ResultVisErr,
                                                                    VisErr,
                                                                },
                                                            },
                                                        };
                                                        use crate::subsystems::oidc::*;

                                                        let ret: Result<Response<Body>, VisErr<loga::Error>> = async {
                                                            let (state, db, head,): (&OidcState, &Pool, Parts,) =
                                                                (&state.oidc_state, &state.db, head,);
                                                            delete_req_session(db, &state.log, &head.headers)
                                                                .await
                                                                .err_internal()?;
                                                            return Ok(
                                                                http::Response::builder()
                                                                    .status(http::StatusCode::TEMPORARY_REDIRECT)
                                                                    .header(
                                                                        http::header::LOCATION,
                                                                        state.public_url.to_string()
                                                                    )
                                                                    .body(body_empty())
                                                                    .unwrap(),
                                                            );
                                                        }.await;
                                                        ret
                                                    }?);
                                                },
                                                c2s::PATH_FILE_UPLOAD => {
                                                    if head.method != Method::POST {
                                                        return Ok(response_404());
                                                    }
                                                    let Some(account) =
                                                        identify_c2s(state, &head.headers).await.err_internal()? else {
                                                            return Ok(response_401());
                                                        };
                                                    let Some(mime_type) =
                                                        head
                                                            .headers
                                                            .get(CONTENT_TYPE)
                                                            .and_then(|v| v.to_str().ok())
                                                            .map(|v| v.to_string()) else {
                                                            return Ok(response_400("Upload requires a content type"));
                                                        };
                                                    return Ok({
                                                        use {
                                                            crate::{
                                                                fsutil::{
                                                                    create_dirs,
                                                                    remove_file_if_exists,
                                                                },
                                                                interface::db::DbAccountId,
                                                                subsystems::files::{
                                                                    account_files_dir,
                                                                    finish_hash,
                                                                    store_file_tmp,
                                                                    tmp_path_in,
                                                                },
                                                            },
                                                            http_body_util::BodyExt,
                                                            loga::{
                                                                DebugDisplay,
                                                                ResultContext,
                                                            },
                                                            sha2::{
                                                                Digest,
                                                                Sha256,
                                                            },
                                                            shared::interface::{
                                                                shared::FileHash,
                                                                wire::c2s::FileUploadRes,
                                                            },
                                                            tokio::io::AsyncWriteExt,
                                                        };

                                                        let ret: Result<Response<Body>, VisErr<loga::Error>> = async {
                                                            let account = DbAccountId(account);
                                                            let dir = account_files_dir(state, account);
                                                            create_dirs(&dir).await.err_internal()?;
                                                            let tmp_path = tmp_path_in(&dir);
                                                            let res = async {
                                                                ta_return!(
                                                                    Option <(FileHash, u64) >,
                                                                    VisErr < loga:: Error >
                                                                );
                                                                let mut file =
                                                                    tokio::fs::File::create(&tmp_path)
                                                                        .await
                                                                        .context_with(
                                                                            "Error creating upload destination",
                                                                            ea!(path = tmp_path.dbg_str())
                                                                        )
                                                                        .err_internal()?;
                                                                let mut hasher = Sha256::new();
                                                                let mut size = 0u64;
                                                                let mut body = body;
                                                                while let Some(frame) = body.frame().await {
                                                                    let frame =
                                                                        frame
                                                                            .context("Error reading upload body")
                                                                            .err_external()?;
                                                                    let Ok(data) = frame.into_data() else {
                                                                        continue;
                                                                    };
                                                                    size += data.len() as u64;
                                                                    if size > state.max_file_size {
                                                                        return Ok(None);
                                                                    }
                                                                    hasher.update(&data);
                                                                    file
                                                                        .write_all(&data)
                                                                        .await
                                                                        .context_with(
                                                                            "Error writing upload",
                                                                            ea!(path = tmp_path.dbg_str())
                                                                        )
                                                                        .err_internal()?;
                                                                }
                                                                file
                                                                    .flush()
                                                                    .await
                                                                    .context_with(
                                                                        "Error flushing upload",
                                                                        ea!(path = tmp_path.dbg_str())
                                                                    )
                                                                    .err_internal()?;
                                                                return Ok(Some((finish_hash(hasher), size)));
                                                            }.await;
                                                            let (hash, size) = match res {
                                                                Ok(Some(r)) => r,
                                                                Ok(None) => {
                                                                    remove_file_if_exists(&tmp_path)
                                                                        .await
                                                                        .err_internal()?;
                                                                    return Ok(
                                                                        Response::builder()
                                                                            .status(http::StatusCode::PAYLOAD_TOO_LARGE)
                                                                            .body(
                                                                                body_full(
                                                                                    format!(
                                                                                        "Files can be at most {} bytes",
                                                                                        state.max_file_size
                                                                                    ).into_bytes()
                                                                                )
                                                                            )
                                                                            .unwrap(),
                                                                    );
                                                                },
                                                                Err(e) => {
                                                                    remove_file_if_exists(&tmp_path)
                                                                        .await
                                                                        .err_internal()?;
                                                                    return Err(e);
                                                                },
                                                            };
                                                            store_file_tmp(
                                                                state,
                                                                account,
                                                                mime_type,
                                                                &tmp_path,
                                                                hash.clone(),
                                                                size
                                                            )
                                                                .await
                                                                .err_internal()?;
                                                            return Ok(response_200_json(&FileUploadRes {
                                                                hash: hash,
                                                                size: size,
                                                            }));
                                                        }.await;
                                                        ret
                                                    }?);
                                                },
                                                "channel_file" => {
                                                    if head.method != Method::GET {
                                                        return Ok(response_404());
                                                    }
                                                    let Some(account) =
                                                        identify_c2s(state, &head.headers).await.err_internal()? else {
                                                            return Ok(response_401());
                                                        };
                                                    let req =
                                                        match <c2s::GetChannelFile as c2s
                                                        ::PathReqTrait>::deserialize_path(
                                                            &path
                                                        ) {
                                                            Ok(r) => r,
                                                            Err(e) => return Ok(response_400(e)),
                                                        };
                                                    return Ok({
                                                        use {
                                                            crate::{
                                                                db,
                                                                dbutil::tx,
                                                                fsutil::{
                                                                    create_dirs,
                                                                    remove_file_if_exists,
                                                                },
                                                                interface::{
                                                                    db::{
                                                                        DbAccountId,
                                                                        DbChannelId,
                                                                        DbIdentity,
                                                                        DbTimeMs,
                                                                    },
                                                                    s2s::{
                                                                        FileAuth,
                                                                        GetFile,
                                                                    },
                                                                },
                                                                subsystems::{
                                                                    files::{
                                                                        file_cache_path,
                                                                        file_path,
                                                                        file_ref,
                                                                        thumbnail_path,
                                                                        tmp_path_in,
                                                                        THUMBNAIL_MIME_TYPE,
                                                                    },
                                                                    messages::{
                                                                        channel_signer,
                                                                        local_channel_get,
                                                                    },
                                                                },
                                                            },
                                                            futures::TryStreamExt,
                                                            http::{
                                                                header::{
                                                                    ACCEPT_RANGES,
                                                                    CONTENT_LENGTH,
                                                                    CONTENT_RANGE,
                                                                    CONTENT_SECURITY_POLICY,
                                                                    RANGE,
                                                                },
                                                                HeaderValue,
                                                                StatusCode,
                                                            },
                                                            http_body_util::{
                                                                BodyExt,
                                                                Full,
                                                                StreamBody,
                                                            },
                                                            htwrap::{
                                                                htreq,
                                                                htserve::responses::response_503,
                                                            },
                                                            hyper::body::Frame,
                                                            jiff::Timestamp,
                                                            loga::{
                                                                DebugDisplay,
                                                                ResultContext,
                                                            },
                                                            shared::interface::wire::c2s::PathReqTrait,
                                                            tokio_util::io::ReaderStream,
                                                        };

                                                        let ret: Result<Response<Body>, VisErr<loga::Error>> = async {
                                                            let account_id = DbAccountId(account);
                                                            let db_owner = DbIdentity(req.channel.identity);
                                                            let db_channel_id =
                                                                DbChannelId(req.channel.channel.clone());
                                                            let db_file_owner = DbIdentity(req.file.identity);
                                                            let thumbnail = req.thumbnail;
                                                            let mut file_headers = HeaderMap::new();
                                                            file_headers.insert(
                                                                X_CONTENT_TYPE_OPTIONS,
                                                                HeaderValue::from_static("nosniff")
                                                            );
                                                            file_headers.insert(
                                                                CONTENT_SECURITY_POLICY,
                                                                HeaderValue::from_static("sandbox; default-src 'none'")
                                                            );
                                                            let Some((local, local_file, cached)) = tx(&state.db, {
                                                                let hash = req.file.hash.0.clone();
                                                                let db_channel_id = db_channel_id.clone();
                                                                move |db_tx| {
                                                                    let Some(local) =
                                                                        local_channel_get(
                                                                            db_tx,
                                                                            account_id,
                                                                            db_owner,
                                                                            &db_channel_id
                                                                        )? else {
                                                                            return Ok(None);
                                                                        };
                                                                    let indexed = good_ormning::sqlite::good_query_opt!(
                                                                        db,
                                                                        r#"select 1 as x from message_file where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and hash = ${string = hash.as_str()} limit ${i64 = 1}"#;
                                                                        db_tx
                                                                    ).map_err(|e| loga::err(e.0))?.is_some();
                                                                    let local_file = if indexed {
                                                                        file_ref(
                                                                            db_tx,
                                                                            account_id,
                                                                            Some(hash.clone())
                                                                        )?
                                                                    } else {
                                                                        None
                                                                    };
                                                                    let access = good_ormning::sqlite::good_query_opt!(
                                                                        db,
                                                                        r#"select 1 as x from file_cache_access where account_id = ${account_id_t = &account_id} and owner = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and hash = ${string = hash.as_str()}"#;
                                                                        db_tx
                                                                    ).map_err(|e| loga::err(e.0))?.is_some();
                                                                    let cached = if access {
                                                                        let cached =
                                                                            good_ormning::sqlite::good_query_opt!(
                                                                                db,
                                                                                r#"select mime_type from file_cache where owner = ${identity_id_t = &db_file_owner} and hash = ${string = hash.as_str()} and thumbnail = ${bool = thumbnail}"#;
                                                                                db_tx
                                                                            ).map_err(|e| loga::err(e.0))?;
                                                                        if cached.is_some() {
                                                                            good_ormning::sqlite::good_query!(
                                                                                db,
                                                                                r#"update file_cache set last_access = ${time_ms_t = &DbTimeMs(Timestamp::now())} where owner = ${identity_id_t = &db_file_owner} and hash = ${string = hash.as_str()} and thumbnail = ${bool = thumbnail}"#;
                                                                                db_tx
                                                                            ).map_err(|e| loga::err(e.0))?;
                                                                        }
                                                                        cached
                                                                    } else {
                                                                        None
                                                                    };
                                                                    return Ok(Some((local, local_file, cached)));
                                                                }
                                                            }).await.err_internal()? else {
                                                                return Ok(response_404());
                                                            };
                                                            if let Some(local_file) = local_file {
                                                                let path =
                                                                    file_path(state, account_id, &req.file.hash);
                                                                let (mime_type, path) = if thumbnail {
                                                                    (
                                                                        THUMBNAIL_MIME_TYPE.to_string(),
                                                                        thumbnail_path(&path)
                                                                    )
                                                                } else {
                                                                    (local_file.mime_type, path)
                                                                };
                                                                return Ok(
                                                                    htwrap::htserve::responses::response_file(
                                                                        &head.headers,
                                                                        &mime_type,
                                                                        &path,
                                                                        true,
                                                                        &file_headers
                                                                    )
                                                                        .await
                                                                        .context_with(
                                                                            "Error serving local file",
                                                                            ea!(path = path.dbg_str())
                                                                        )
                                                                        .err_internal()?,
                                                                );
                                                            }
                                                            if let Some(mime_type) = cached {
                                                                let path =
                                                                    file_cache_path(
                                                                        state,
                                                                        req.file.identity,
                                                                        &req.file.hash.0,
                                                                        thumbnail
                                                                    );
                                                                return Ok(
                                                                    htwrap::htserve::responses::response_file(
                                                                        &head.headers,
                                                                        &mime_type,
                                                                        &path,
                                                                        true,
                                                                        &file_headers
                                                                    )
                                                                        .await
                                                                        .context_with(
                                                                            "Error serving cached file",
                                                                            ea!(path = path.dbg_str())
                                                                        )
                                                                        .err_internal()?,
                                                                );
                                                            }
                                                            let signer =
                                                                channel_signer(state, account, &local)
                                                                    .await
                                                                    .err_internal()?;
                                                            let s2s_req = GetFile {
                                                                auth: FileAuth::Channel(req.channel.clone()),
                                                                hash: req.file.hash.clone(),
                                                                thumbnail: thumbnail,
                                                            };
                                                            let auth_headers =
                                                                state
                                                                    .s2s_client
                                                                    .opt_auth_headers(
                                                                        req.file.identity,
                                                                        signer.as_ref()
                                                                    )
                                                                    .await
                                                                    .err_internal()?;
                                                            let (conn, url) =
                                                                state
                                                                    .s2s_client
                                                                    .connect(
                                                                        req.file.identity,
                                                                        &format!("/{}", s2s_req.serialize_path())
                                                                    )
                                                                    .await
                                                                    .err_internal()?;
                                                            let mut builder =
                                                                Request::builder()
                                                                    .method(Method::GET)
                                                                    .uri(url.clone());
                                                            for (k, v) in &auth_headers {
                                                                builder = builder.header(k, v);
                                                            }
                                                            if let Some(range) = head.headers.get(RANGE) {
                                                                builder = builder.header(RANGE, range.clone());
                                                            }
                                                            let s2s_request =
                                                                builder.body(Full::new(Bytes::new())).unwrap();
                                                            let (meta_tx, meta_rx) = tokio::sync::oneshot::channel();
                                                            let (writer, reader) = tokio::io::duplex(64 * 1024);
                                                            tokio::spawn({
                                                                let log = state.log.clone();
                                                                let limits = state.s2s_client.limits;
                                                                async move {
                                                                    let mut conn = conn;
                                                                    match htreq::send(
                                                                        &log,
                                                                        limits,
                                                                        &mut conn,
                                                                        s2s_request
                                                                    ).await {
                                                                        Err(e) => {
                                                                            _ = meta_tx.send(Err(e));
                                                                        },
                                                                        Ok((status, headers, continue_send)) => {
                                                                            if meta_tx
                                                                                .send(Ok((status, headers)))
                                                                                .is_err() {
                                                                                return;
                                                                            }
                                                                            if let Err(e) =
                                                                                htreq::receive_stream(
                                                                                    limits,
                                                                                    continue_send,
                                                                                    writer
                                                                                ).await {
                                                                                log.log_err(
                                                                                    loga::DEBUG,
                                                                                    e.context(
                                                                                        "Error streaming proxied file"
                                                                                    )
                                                                                );
                                                                            }
                                                                        },
                                                                    }
                                                                }
                                                            });
                                                            let (status, resp_headers) =
                                                                match meta_rx
                                                                    .await
                                                                    .context("Proxy task ended without a response")
                                                                    .err_internal()? {
                                                                    Ok(r) => r,
                                                                    Err(e) => {
                                                                        state
                                                                            .log
                                                                            .log_err(
                                                                                loga::DEBUG,
                                                                                e.context_with(
                                                                                    "Error requesting file from owner",
                                                                                    ea!(url = url)
                                                                                )
                                                                            );
                                                                        return Ok(response_503());
                                                                    },
                                                                };
                                                            if !status.is_success() {
                                                                if let Some(signer) = &signer {
                                                                    state
                                                                        .s2s_client
                                                                        .tokens
                                                                        .invalidate(
                                                                            &(
                                                                                req.file.identity,
                                                                                crate
                                                                                ::subsystems
                                                                                ::s2s_client
                                                                                ::signer_identity(
                                                                                    signer
                                                                                ).err_internal()?
                                                                            ),
                                                                        )
                                                                        .await;
                                                                }
                                                                state
                                                                    .log
                                                                    .log_with(
                                                                        loga::DEBUG,
                                                                        "Owner refused file request",
                                                                        ea!(url = url, status = status)
                                                                    );
                                                                match status {
                                                                    StatusCode::NOT_FOUND |
                                                                    StatusCode::FORBIDDEN |
                                                                    StatusCode::RANGE_NOT_SATISFIABLE => {
                                                                        return Ok(
                                                                            Response::builder()
                                                                                .status(status)
                                                                                .body(body_full(vec![]))
                                                                                .unwrap()
                                                                        );
                                                                    },
                                                                    _ => return Ok(response_503()),
                                                                }
                                                            }
                                                            tx(&state.db, {
                                                                let hash = req.file.hash.0.clone();
                                                                let db_channel_id = db_channel_id.clone();
                                                                move |db_tx| {
                                                                    good_ormning::sqlite::good_query!(
                                                                        db,
                                                                        r#"insert into file_cache_access (account_id, owner, channel, hash) values (${account_id_t = &account_id}, ${identity_id_t = &db_owner}, ${channel_id_t = &db_channel_id}, ${string = hash.as_str()}) on conflict do nothing"#;
                                                                        db_tx
                                                                    ).map_err(|e| loga::err(e.0))?;
                                                                    return Ok(());
                                                                }
                                                            }).await.err_internal()?;
                                                            let fill_key =
                                                                (
                                                                    req.file.identity,
                                                                    req.file.hash.0.clone(),
                                                                    thumbnail
                                                                );
                                                            if state
                                                                .file_cache_fills
                                                                .lock()
                                                                .unwrap()
                                                                .insert(fill_key.clone()) {
                                                                let state = state.clone();
                                                                let mime_type =
                                                                    resp_headers
                                                                        .get(CONTENT_TYPE)
                                                                        .and_then(|v| v.to_str().ok())
                                                                        .context_with(
                                                                            "Owner file response missing content type",
                                                                            ea!(url = url)
                                                                        )
                                                                        .err_internal()?
                                                                        .to_string();
                                                                let file = req.file.clone();
                                                                tokio::spawn(async move {
                                                                    if let Err(e) = async {
                                                                        ta_return!((), loga::Error);
                                                                        let dest =
                                                                            file_cache_path(
                                                                                &state,
                                                                                file.identity,
                                                                                &file.hash.0,
                                                                                thumbnail
                                                                            );
                                                                        let dir = dest.parent().unwrap().to_path_buf();
                                                                        create_dirs(&dir).await?;
                                                                        let tmp_path = tmp_path_in(&dir);
                                                                        let downloaded =
                                                                            state
                                                                                .s2s_client
                                                                                .get_file_to_disk(
                                                                                    file.identity,
                                                                                    signer.as_ref(),
                                                                                    &s2s_req,
                                                                                    &tmp_path,
                                                                                    state.file_cache_size
                                                                                )
                                                                                .await;
                                                                        let (hash, size) = match downloaded {
                                                                            Ok(d) => d,
                                                                            Err(e) => {
                                                                                remove_file_if_exists(
                                                                                    &tmp_path
                                                                                ).await?;
                                                                                return Err(e);
                                                                            },
                                                                        };
                                                                        if !thumbnail && hash != file.hash {
                                                                            remove_file_if_exists(&tmp_path).await?;
                                                                            return Err(
                                                                                loga::err_with(
                                                                                    "Owner sent file contents that don't match the hash",
                                                                                    ea!(want = file.hash, got = hash),
                                                                                ),
                                                                            );
                                                                        }
                                                                        tokio::fs::rename(&tmp_path, &dest)
                                                                            .await
                                                                            .context_with(
                                                                                "Error moving cached file into place",
                                                                                ea!(path = dest.dbg_str())
                                                                            )?;
                                                                        let size =
                                                                            i64::try_from(
                                                                                size
                                                                            ).context(
                                                                                "Cached file size out of range"
                                                                            )?;
                                                                        let cache_size =
                                                                            i64::try_from(
                                                                                state.file_cache_size
                                                                            ).context("Cache size out of range")?;
                                                                        let evicted =
                                                                            tx(&state.db, move |db_tx| {
                                                                                let db_file_owner =
                                                                                    DbIdentity(file.identity);
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"insert into file_cache (owner, hash, thumbnail, mime_type, size, last_access) values (${identity_id_t = &db_file_owner}, ${string = file.hash.0.as_str()}, ${bool = thumbnail}, ${string = mime_type.as_str()}, ${i64 = size}, ${time_ms_t = &DbTimeMs(Timestamp::now())}) on conflict do update set size = excluded.size, last_access = excluded.last_access"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                                let mut total = 0i64;
                                                                                let mut evicted = vec![];
                                                                                for r in good_ormning
                                                                                ::sqlite
                                                                                ::good_query_many!(
                                                                                    db,
                                                                                    r#"select owner, hash, thumbnail, size from file_cache order by last_access desc"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))? {
                                                                                    total =
                                                                                        total.saturating_add(r.size);
                                                                                    if total <= cache_size {
                                                                                        continue;
                                                                                    }
                                                                                    good_ormning::sqlite::good_query!(
                                                                                        db,
                                                                                        r#"delete from file_cache where owner = ${identity_id_t = &r.owner} and hash = ${string = r.hash.as_str()} and thumbnail = ${bool = r.thumbnail}"#;
                                                                                        db_tx
                                                                                    ).map_err(|e| loga::err(e.0))?;
                                                                                    let remaining =
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query_opt!(
                                                                                            db,
                                                                                            r#"select 1 as x from file_cache where owner = ${identity_id_t = &r.owner} and hash = ${string = r.hash.as_str()}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    if remaining.is_none() {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"delete from file_cache_access where owner = ${identity_id_t = &r.owner} and hash = ${string = r.hash.as_str()}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    evicted.push(
                                                                                        (r.owner.0, r.hash, r.thumbnail)
                                                                                    );
                                                                                }
                                                                                return Ok(evicted);
                                                                            })
                                                                                .await
                                                                                .context(
                                                                                    "Error recording cached file"
                                                                                )?;
                                                                        for (owner, hash, thumbnail) in evicted {
                                                                            remove_file_if_exists(
                                                                                &file_cache_path(
                                                                                    &state,
                                                                                    owner,
                                                                                    &hash,
                                                                                    thumbnail
                                                                                )
                                                                            ).await?;
                                                                        }
                                                                        return Ok(());
                                                                    }.await {
                                                                        state
                                                                            .log
                                                                            .log_err(
                                                                                loga::DEBUG,
                                                                                e.context("Error filling file cache")
                                                                            );
                                                                    }
                                                                    state
                                                                        .file_cache_fills
                                                                        .lock()
                                                                        .unwrap()
                                                                        .remove(&fill_key);
                                                                });
                                                            }
                                                            let mut resp = Response::builder().status(status);
                                                            for h in [
                                                                CONTENT_TYPE,
                                                                CONTENT_LENGTH,
                                                                CONTENT_RANGE,
                                                                ACCEPT_RANGES
                                                            ] {
                                                                if let Some(v) = resp_headers.get(&h) {
                                                                    resp = resp.header(h, v.clone());
                                                                }
                                                            }
                                                            for (k, v) in &file_headers {
                                                                resp = resp.header(k, v.clone());
                                                            }
                                                            resp =
                                                                resp.header(
                                                                    CACHE_CONTROL,
                                                                    crate
                                                                    ::subsystems
                                                                    ::s2s_error
                                                                    ::CachePolicy
                                                                    ::Immutable.header_value()
                                                                );
                                                            return Ok(
                                                                resp
                                                                    .body(
                                                                        StreamBody::new(
                                                                            ReaderStream::new(
                                                                                reader
                                                                            ).map_ok(Frame::data)
                                                                        ).boxed()
                                                                    )
                                                                    .unwrap(),
                                                            );
                                                        }.await;
                                                        ret
                                                    }?);
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
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::DbAccountId,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::NotificationRegister,
                                                                        },
                                                                    };

                                                                    let ret: Result<(), loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (
                                                                                &State,
                                                                                AccountId,
                                                                                NotificationRegister,
                                                                            ) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let endpoint =
                                                                            req
                                                                                .data
                                                                                .get("endpoint")
                                                                                .and_then(|v| v.as_str())
                                                                                .context(
                                                                                    "Push subscription missing 'endpoint' field"
                                                                                )?
                                                                                .to_string();
                                                                        let data =
                                                                            serde_json::to_string(&req.data).unwrap();
                                                                        tx(&state.db, move |db_tx| {
                                                                            good_ormning::sqlite::good_query!(
                                                                                db,
                                                                                r#"insert into push_subscription (account_id, endpoint, data) values (${account_id_t = &account_id}, ${string = endpoint.as_str()}, ${string = data.as_str()}) on conflict (account_id, endpoint) do update set data = excluded.data"#;
                                                                                db_tx
                                                                            ).map_err(|e| loga::err(e.0))?;
                                                                            return Ok(());
                                                                        })
                                                                            .await
                                                                            .context(
                                                                                "Error registering push subscription"
                                                                            )?;
                                                                        return Ok(());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::IdentityCreate(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbIdentity,
                                                                                DbIdentitySecret,
                                                                            },
                                                                            subsystems::files::file_ref,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::{
                                                                                IdentityCreate,
                                                                                IdentityRes,
                                                                            },
                                                                        },
                                                                        spaghettinuum_native::utils::identity_secret::new_local_identity_secret,
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<IdentityRes, loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (&State, AccountId, IdentityCreate,) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let secret = new_local_identity_secret();
                                                                        let identity =
                                                                            secret
                                                                                .identity()
                                                                                .context(
                                                                                    "Generated identity secret produced no public key"
                                                                                )?;
                                                                        let idem =
                                                                            req
                                                                                .idem
                                                                                .clone()
                                                                                .unwrap_or_else(
                                                                                    || identity.to_string()
                                                                                );
                                                                        let row = tx(&state.db, move |db_tx| {
                                                                            let existing =
                                                                                good_ormning::sqlite::good_query_opt!(
                                                                                    db,
                                                                                    r#"select id, idem, name, description, shown_name, shown_description, portrait from identity where account_id = ${account_id_t = &account_id} and idem = ${string = idem.as_str()} and soft_deleted_at is null"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            if let Some(existing) = existing {
                                                                                return Ok(IdentityRes {
                                                                                    id: existing.id.0,
                                                                                    idem: opt_idem(existing.idem),
                                                                                    name: existing.name,
                                                                                    description: existing.description,
                                                                                    shown_name: existing.shown_name,
                                                                                    shown_description: existing.shown_description,
                                                                                    portrait: file_ref(
                                                                                        db_tx,
                                                                                        account_id,
                                                                                        existing.portrait
                                                                                    )?,
                                                                                });
                                                                            }
                                                                            let db_identity = DbIdentity(identity);
                                                                            let db_secret = DbIdentitySecret(secret);
                                                                            good_ormning::sqlite::good_query!(
                                                                                db,
                                                                                r#"insert into identity (account_id, id, idem, name, description, shown_name, shown_description, secret) values (${account_id_t = &account_id}, ${identity_id_t = &db_identity}, ${string = idem.as_str()}, ${string = req.name.as_str()}, ${string = req.description.as_str()}, ${opt string = req.shown_name.as_deref()}, ${opt string = req.shown_description.as_deref()}, ${identity_secret_t = &db_secret})"#;
                                                                                db_tx
                                                                            ).map_err(|e| loga::err(e.0))?;
                                                                            return Ok(IdentityRes {
                                                                                id: identity,
                                                                                idem: opt_idem(idem),
                                                                                name: req.name,
                                                                                description: req.description,
                                                                                shown_name: req.shown_name,
                                                                                shown_description: req.shown_description,
                                                                                portrait: None,
                                                                            });
                                                                        }).await.context("Error inserting identity")?;
                                                                        crate
                                                                        ::subsystems
                                                                        ::spagh_publish
                                                                        ::publish_identity(
                                                                            state,
                                                                            row.id
                                                                        ).await?;
                                                                        return Ok(row);
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::IdentityModify(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbIdentity,
                                                                            },
                                                                            subsystems::files::file_ref,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::{
                                                                                IdentityModify,
                                                                                IdentityRes,
                                                                            },
                                                                        },
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<IdentityRes, loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (&State, AccountId, IdentityModify,) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let db_identity = DbIdentity(req.id);
                                                                        let portrait =
                                                                            resolve_modify_portrait(
                                                                                state,
                                                                                account_id,
                                                                                &req.portrait
                                                                            ).await?;
                                                                        let row = tx(&state.db, move |db_tx| {
                                                                            require_owned_identity(
                                                                                db_tx,
                                                                                account_id,
                                                                                db_identity
                                                                            )?;
                                                                            if let Some(v) = &req.name {
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update identity set name = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(v) = &req.description {
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update identity set description = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(m) = &req.shown_name {
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update identity set shown_name = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(m) = &req.shown_description {
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update identity set shown_description = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if !matches!(
                                                                                portrait,
                                                                                PortraitChange::Keep
                                                                            ) {
                                                                                let current =
                                                                                    good_ormning
                                                                                    ::sqlite
                                                                                    ::good_query_opt!(
                                                                                        db,
                                                                                        r#"select portrait from identity where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
                                                                                        db_tx
                                                                                    )
                                                                                        .map_err(|e| loga::err(e.0))?
                                                                                        .flatten();
                                                                                let next = portrait.apply(current);
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update identity set portrait = ${opt string = next.as_deref()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            let row =
                                                                                good_ormning::sqlite::good_query_opt!(
                                                                                    db,
                                                                                    r#"select id, idem, name, description, shown_name, shown_description, portrait from identity where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity} and soft_deleted_at is null"#;
                                                                                    db_tx
                                                                                )
                                                                                    .map_err(|e| loga::err(e.0))?
                                                                                    .context("No such identity")?;
                                                                            return Ok(IdentityRes {
                                                                                id: row.id.0,
                                                                                idem: opt_idem(row.idem),
                                                                                name: row.name,
                                                                                description: row.description,
                                                                                shown_name: row.shown_name,
                                                                                shown_description: row.shown_description,
                                                                                portrait: file_ref(
                                                                                    db_tx,
                                                                                    account_id,
                                                                                    row.portrait
                                                                                )?,
                                                                            });
                                                                        }).await.context("Error modifying identity")?;
                                                                        return Ok(row);
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::IdentityDelete(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbIdentity,
                                                                            },
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::IdentityDelete,
                                                                        },
                                                                    };

                                                                    let ret: Result<(), loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (&State, AccountId, IdentityDelete,) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let db_identity = DbIdentity(req.id);
                                                                        let now = jiff::Timestamp::now();
                                                                        tx(&state.db, move |db_tx| {
                                                                            good_ormning::sqlite::good_query!(
                                                                                db,
                                                                                r#"update identity set soft_deleted_at = ${time_ms_t = &crate::interface::db::DbTimeMs(now)} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
                                                                                db_tx
                                                                            ).map_err(|e| loga::err(e.0))?;
                                                                            return Ok(());
                                                                        }).await.context("Error deleting identity")?;
                                                                        {
                                                                            use {
                                                                                crate::State,
                                                                                loga::{
                                                                                    ea,
                                                                                    ResultContext,
                                                                                },
                                                                                spaghettinuum::interface::identity::Identity,
                                                                            };

                                                                            let ret: Result<(), loga::Error> = async {
                                                                                let (state, identity,):
                                                                                    (&State, Identity,) =
                                                                                    (state, req.id,);
                                                                                state
                                                                                    .spagh_publish
                                                                                    .certs
                                                                                    .0
                                                                                    .write()
                                                                                    .unwrap()
                                                                                    .remove(&identity);
                                                                                state
                                                                                    .spagh_publish
                                                                                    .publisher
                                                                                    .clear_identity(&identity)
                                                                                    .await
                                                                                    .context_with(
                                                                                        "Error clearing published identity",
                                                                                        ea!(identity = identity)
                                                                                    )?;
                                                                                return Ok(());
                                                                            }.await;
                                                                            ret
                                                                        }?;
                                                                        return Ok(());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::IdentityList(_, _) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::DbAccountId,
                                                                            subsystems::files::file_ref,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::IdentityRes,
                                                                        },
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<Vec<IdentityRes>, loga::Error> =
                                                                        async {
                                                                            let (state, account,):
                                                                                (&State, AccountId,) =
                                                                                (state, account,);
                                                                            let account_id = DbAccountId(account);
                                                                            return Ok(
                                                                                tx(&state.db, move |db_tx| {
                                                                                    let rows =
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query_many!(
                                                                                            db,
                                                                                            r#"select id, idem, name, description, shown_name, shown_description, portrait from identity where account_id = ${account_id_t = &account_id} and soft_deleted_at is null order by id"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    let mut out = vec![];
                                                                                    for r in rows {
                                                                                        out.push(IdentityRes {
                                                                                            id: r.id.0,
                                                                                            idem: opt_idem(r.idem),
                                                                                            name: r.name,
                                                                                            description: r.description,
                                                                                            shown_name: r.shown_name,
                                                                                            shown_description: r.shown_description,
                                                                                            portrait: file_ref(
                                                                                                db_tx,
                                                                                                account_id,
                                                                                                r.portrait
                                                                                            )?,
                                                                                        });
                                                                                    }
                                                                                    return Ok(out);
                                                                                })
                                                                                    .await
                                                                                    .context(
                                                                                        "Error listing identities"
                                                                                    )?
                                                                            );
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::IdentityInviteCreate(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbIdentity,
                                                                            },
                                                                            subsystems::messages::i64_to_u64,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::{
                                                                                AccountId,
                                                                                IdentityInviteId,
                                                                                IdentityInviteToken,
                                                                                QualifiedIdentityInviteToken,
                                                                            },
                                                                            wire::c2s::{
                                                                                IdentityInviteCreate,
                                                                                IdentityInviteRes,
                                                                            },
                                                                        },
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<IdentityInviteRes, loga::Error> =
                                                                        async {
                                                                            let (state, account, req,):
                                                                                (
                                                                                    &State,
                                                                                    AccountId,
                                                                                    IdentityInviteCreate,
                                                                                ) =
                                                                                (state, account, r,);
                                                                            let account_id = DbAccountId(account);
                                                                            let identity = req.identity;
                                                                            let db_identity = DbIdentity(identity);
                                                                            let idem =
                                                                                req.idem.clone().unwrap_or_default();
                                                                            let token = random_token();
                                                                            return Ok(
                                                                                tx(&state.db, move |db_tx| {
                                                                                    require_owned_identity(
                                                                                        db_tx,
                                                                                        account_id,
                                                                                        db_identity
                                                                                    )?;
                                                                                    if !idem.is_empty() {
                                                                                        let existing =
                                                                                            good_ormning
                                                                                            ::sqlite
                                                                                            ::good_query_opt!(
                                                                                                db,
                                                                                                r#"select rowid, token, name, description, shown_name, shown_description, single_use, expiry from identity_invite where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_identity} and idem = ${string = idem.as_str()}"#;
                                                                                                db_tx
                                                                                            ).map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?;
                                                                                        if let Some(e) = existing {
                                                                                            return Ok(
                                                                                                IdentityInviteRes {
                                                                                                    id: IdentityInviteId(
                                                                                                        i64_to_u64(
                                                                                                            e.rowid
                                                                                                        )
                                                                                                    ),
                                                                                                    token: QualifiedIdentityInviteToken {
                                                                                                        identity: identity,
                                                                                                        token: IdentityInviteToken(
                                                                                                            e.token
                                                                                                        ),
                                                                                                    },
                                                                                                    name: e.name,
                                                                                                    description: e.description,
                                                                                                    shown_name: e.shown_name,
                                                                                                    shown_description: e.shown_description,
                                                                                                    single_use: e.single_use,
                                                                                                    expiry: e
                                                                                                        .expiry
                                                                                                        .map(|t| t.0),
                                                                                                }
                                                                                            );
                                                                                        }
                                                                                    }
                                                                                    good_ormning::sqlite::good_query!(
                                                                                        db,
                                                                                        r#"insert into identity_invite (account_id, identity, idem, token, name, description, shown_name, shown_description, single_use, expiry, used_count) values (${account_id_t = &account_id}, ${identity_id_t = &db_identity}, ${string = idem.as_str()}, ${string = token.as_str()}, ${string = req.name.as_str()}, ${string = req.description.as_str()}, ${opt string = req.shown_name.as_deref()}, ${opt string = req.shown_description.as_deref()}, ${bool = req.single_use}, ${opt time_ms_t = req.expiry.map(crate::interface::db::DbTimeMs).as_ref()}, ${i64 = 0})"#;
                                                                                        db_tx
                                                                                    ).map_err(|e| loga::err(e.0))?;
                                                                                    return Ok(IdentityInviteRes {
                                                                                        id: IdentityInviteId(
                                                                                            i64_to_u64(
                                                                                                db_tx
                                                                                                    .0
                                                                                                    .last_insert_rowid()
                                                                                            )
                                                                                        ),
                                                                                        token: QualifiedIdentityInviteToken {
                                                                                            identity: identity,
                                                                                            token: IdentityInviteToken(
                                                                                                token
                                                                                            ),
                                                                                        },
                                                                                        name: req.name,
                                                                                        description: req.description,
                                                                                        shown_name: req.shown_name,
                                                                                        shown_description: req.shown_description,
                                                                                        single_use: req.single_use,
                                                                                        expiry: req.expiry,
                                                                                    });
                                                                                })
                                                                                    .await
                                                                                    .context(
                                                                                        "Error inserting identity invite"
                                                                                    )?
                                                                            );
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::IdentityInviteModify(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::DbAccountId,
                                                                            subsystems::messages::i64_to_u64,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::{
                                                                                AccountId,
                                                                                IdentityInviteId,
                                                                                IdentityInviteToken,
                                                                                QualifiedIdentityInviteToken,
                                                                            },
                                                                            wire::c2s::{
                                                                                IdentityInviteModify,
                                                                                IdentityInviteRes,
                                                                            },
                                                                        },
                                                                    };

                                                                    let ret: Result<IdentityInviteRes, loga::Error> =
                                                                        async {
                                                                            let (state, account, req,):
                                                                                (
                                                                                    &State,
                                                                                    AccountId,
                                                                                    IdentityInviteModify,
                                                                                ) =
                                                                                (state, account, r,);
                                                                            let account_id = DbAccountId(account);
                                                                            let rowid =
                                                                                crate
                                                                                ::subsystems
                                                                                ::messages
                                                                                ::u64_to_i64(
                                                                                    req.id.0
                                                                                );
                                                                            let row =
                                                                                tx(&state.db, move |db_tx| {
                                                                                    if let Some(v) = &req.name {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update identity_invite set name = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    if let Some(v) = &req.description {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update identity_invite set description = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    if let Some(m) = &req.shown_name {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update identity_invite set shown_name = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    if let Some(m) =
                                                                                        &req.shown_description {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update identity_invite set shown_description = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    if let Some(v) = req.single_use {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update identity_invite set single_use = ${bool = v} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    if let Some(m) = req.expiry {
                                                                                        let v = m.value;
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update identity_invite set expiry = ${opt time_ms_t = v.map(crate::interface::db::DbTimeMs).as_ref()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    return Ok(
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query_opt!(
                                                                                            db,
                                                                                            r#"select rowid, identity, token, name, description, shown_name, shown_description, single_use, expiry from identity_invite where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        )
                                                                                            .map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?
                                                                                            .context(
                                                                                                "No such identity invite"
                                                                                            )?
                                                                                    );
                                                                                })
                                                                                    .await
                                                                                    .context(
                                                                                        "Error modifying identity invite"
                                                                                    )?;
                                                                            return Ok(IdentityInviteRes {
                                                                                id: IdentityInviteId(
                                                                                    i64_to_u64(row.rowid)
                                                                                ),
                                                                                token: QualifiedIdentityInviteToken {
                                                                                    identity: row.identity.0,
                                                                                    token: IdentityInviteToken(
                                                                                        row.token
                                                                                    ),
                                                                                },
                                                                                name: row.name,
                                                                                description: row.description,
                                                                                shown_name: row.shown_name,
                                                                                shown_description: row.shown_description,
                                                                                single_use: row.single_use,
                                                                                expiry: row.expiry.map(|t| t.0),
                                                                            });
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::IdentityInviteDelete(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::DbAccountId,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::IdentityInviteDelete,
                                                                        },
                                                                    };

                                                                    let ret: Result<(), loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (
                                                                                &State,
                                                                                AccountId,
                                                                                IdentityInviteDelete,
                                                                            ) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let rowid =
                                                                            crate::subsystems::messages::u64_to_i64(
                                                                                req.id.0
                                                                            );
                                                                        tx(&state.db, move |db_tx| {
                                                                            good_ormning::sqlite::good_query!(
                                                                                db,
                                                                                r#"delete from identity_invite where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                db_tx
                                                                            ).map_err(|e| loga::err(e.0))?;
                                                                            return Ok(());
                                                                        })
                                                                            .await
                                                                            .context(
                                                                                "Error deleting identity invite"
                                                                            )?;
                                                                        return Ok(());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::IdentityInviteList(_, _) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::DbAccountId,
                                                                            subsystems::messages::i64_to_u64,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::{
                                                                                AccountId,
                                                                                IdentityInviteId,
                                                                                IdentityInviteToken,
                                                                                QualifiedIdentityInviteToken,
                                                                            },
                                                                            wire::c2s::IdentityInviteRes,
                                                                        },
                                                                    };

                                                                    let ret:
                                                                        Result<Vec<IdentityInviteRes>, loga::Error> =
                                                                        async {
                                                                            let (state, account,):
                                                                                (&State, AccountId,) =
                                                                                (state, account,);
                                                                            let account_id = DbAccountId(account);
                                                                            let rows =
                                                                                tx(&state.db, move |db_tx| {
                                                                                    return Ok(
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query_many!(
                                                                                            db,
                                                                                            r#"select rowid, identity, token, name, description, shown_name, shown_description, single_use, expiry from identity_invite where account_id = ${account_id_t = &account_id} order by rowid"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?
                                                                                    );
                                                                                })
                                                                                    .await
                                                                                    .context(
                                                                                        "Error listing identity invites"
                                                                                    )?;
                                                                            return Ok(
                                                                                rows
                                                                                    .into_iter()
                                                                                    .map(|r| IdentityInviteRes {
                                                                                        id: IdentityInviteId(
                                                                                            i64_to_u64(r.rowid)
                                                                                        ),
                                                                                        token: QualifiedIdentityInviteToken {
                                                                                            identity: r.identity.0,
                                                                                            token: IdentityInviteToken(
                                                                                                r.token
                                                                                            ),
                                                                                        },
                                                                                        name: r.name,
                                                                                        description: r.description,
                                                                                        shown_name: r.shown_name,
                                                                                        shown_description: r.shown_description,
                                                                                        single_use: r.single_use,
                                                                                        expiry: r.expiry.map(|t| t.0),
                                                                                    })
                                                                                    .collect()
                                                                            );
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelGroupCreate(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::DbAccountId,
                                                                            subsystems::messages::i64_to_u64,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::{
                                                                                AccountId,
                                                                                ChannelGroupId,
                                                                            },
                                                                            wire::c2s::{
                                                                                ChannelGroupCreate,
                                                                                ChannelGroupRes,
                                                                            },
                                                                        },
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<ChannelGroupRes, loga::Error> =
                                                                        async {
                                                                            let (state, account, req,):
                                                                                (
                                                                                    &State,
                                                                                    AccountId,
                                                                                    ChannelGroupCreate,
                                                                                ) =
                                                                                (state, account, r,);
                                                                            let account_id = DbAccountId(account);
                                                                            let idem =
                                                                                req.idem.clone().unwrap_or_default();
                                                                            let row =
                                                                                tx(&state.db, move |db_tx| {
                                                                                    if !idem.is_empty() {
                                                                                        let existing =
                                                                                            good_ormning
                                                                                            ::sqlite
                                                                                            ::good_query_opt!(
                                                                                                db,
                                                                                                r#"select rowid, idem, name, description from channelgroup where account_id = ${account_id_t = &account_id} and idem = ${string = idem.as_str()}"#;
                                                                                                db_tx
                                                                                            ).map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?;
                                                                                        if let Some(existing) =
                                                                                            existing {
                                                                                            return Ok(
                                                                                                (
                                                                                                    existing.rowid,
                                                                                                    existing.idem,
                                                                                                    existing.name,
                                                                                                    existing.description
                                                                                                )
                                                                                            );
                                                                                        }
                                                                                    }
                                                                                    good_ormning::sqlite::good_query!(
                                                                                        db,
                                                                                        r#"insert into channelgroup (account_id, idem, name, description) values (${account_id_t = &account_id}, ${string = idem.as_str()}, ${string = req.name.as_str()}, ${string = req.description.as_str()})"#;
                                                                                        db_tx
                                                                                    ).map_err(|e| loga::err(e.0))?;
                                                                                    return Ok(
                                                                                        (
                                                                                            db_tx.0.last_insert_rowid(),
                                                                                            idem,
                                                                                            req.name,
                                                                                            req.description
                                                                                        )
                                                                                    );
                                                                                })
                                                                                    .await
                                                                                    .context(
                                                                                        "Error inserting channel group"
                                                                                    )?;
                                                                            return Ok(ChannelGroupRes {
                                                                                id: ChannelGroupId(i64_to_u64(row.0)),
                                                                                idem: opt_idem(row.1),
                                                                                name: row.2,
                                                                                description: row.3,
                                                                            });
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelGroupModify(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbChannelGroupId,
                                                                            },
                                                                            subsystems::messages::i64_to_u64,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::{
                                                                                AccountId,
                                                                                ChannelGroupId,
                                                                            },
                                                                            wire::c2s::{
                                                                                ChannelGroupModify,
                                                                                ChannelGroupRes,
                                                                            },
                                                                        },
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<ChannelGroupRes, loga::Error> =
                                                                        async {
                                                                            let (state, account, req,):
                                                                                (
                                                                                    &State,
                                                                                    AccountId,
                                                                                    ChannelGroupModify,
                                                                                ) =
                                                                                (state, account, r,);
                                                                            let account_id = DbAccountId(account);
                                                                            let gid = DbChannelGroupId(req.id);
                                                                            let row =
                                                                                tx(&state.db, move |db_tx| {
                                                                                    if let Some(v) = &req.name {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update channelgroup set name = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and rowid = ${channelgroup_id_t = &gid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    if let Some(v) = &req.description {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update channelgroup set description = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and rowid = ${channelgroup_id_t = &gid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    return Ok(
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query_opt!(
                                                                                            db,
                                                                                            r#"select rowid, idem, name, description from channelgroup where account_id = ${account_id_t = &account_id} and rowid = ${channelgroup_id_t = &gid}"#;
                                                                                            db_tx
                                                                                        )
                                                                                            .map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?
                                                                                            .context(
                                                                                                "No such channel group"
                                                                                            )?
                                                                                    );
                                                                                })
                                                                                    .await
                                                                                    .context(
                                                                                        "Error modifying channel group"
                                                                                    )?;
                                                                            return Ok(ChannelGroupRes {
                                                                                id: ChannelGroupId(
                                                                                    i64_to_u64(row.rowid)
                                                                                ),
                                                                                idem: opt_idem(row.idem),
                                                                                name: row.name,
                                                                                description: row.description,
                                                                            });
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelGroupDelete(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbChannelGroupId,
                                                                            },
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::ChannelGroupDelete,
                                                                        },
                                                                    };

                                                                    let ret: Result<(), loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (&State, AccountId, ChannelGroupDelete,) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let gid = DbChannelGroupId(req.id);
                                                                        tx(&state.db, move |db_tx| {
                                                                            good_ormning::sqlite::good_query!(
                                                                                db,
                                                                                r#"update channel set channel_group = ${opt channelgroup_id_t = None} where account_id = ${account_id_t = &account_id} and channel_group = ${channelgroup_id_t = &gid}"#;
                                                                                db_tx
                                                                            ).map_err(|e| loga::err(e.0))?;
                                                                            good_ormning::sqlite::good_query!(
                                                                                db,
                                                                                r#"delete from channelgroup where account_id = ${account_id_t = &account_id} and rowid = ${channelgroup_id_t = &gid}"#;
                                                                                db_tx
                                                                            ).map_err(|e| loga::err(e.0))?;
                                                                            return Ok(());
                                                                        })
                                                                            .await
                                                                            .context("Error deleting channel group")?;
                                                                        return Ok(());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelGroupList(_, _) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::DbAccountId,
                                                                            subsystems::messages::i64_to_u64,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::{
                                                                                AccountId,
                                                                                ChannelGroupId,
                                                                            },
                                                                            wire::c2s::ChannelGroupRes,
                                                                        },
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<Vec<ChannelGroupRes>, loga::Error> =
                                                                        async {
                                                                            let (state, account,):
                                                                                (&State, AccountId,) =
                                                                                (state, account,);
                                                                            let account_id = DbAccountId(account);
                                                                            let rows =
                                                                                tx(&state.db, move |db_tx| {
                                                                                    return Ok(
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query_many!(
                                                                                            db,
                                                                                            r#"select rowid, idem, name, description from channelgroup where account_id = ${account_id_t = &account_id} order by rowid"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?
                                                                                    );
                                                                                })
                                                                                    .await
                                                                                    .context(
                                                                                        "Error listing channel groups"
                                                                                    )?;
                                                                            return Ok(
                                                                                rows
                                                                                    .into_iter()
                                                                                    .map(|r| ChannelGroupRes {
                                                                                        id: ChannelGroupId(
                                                                                            i64_to_u64(r.rowid)
                                                                                        ),
                                                                                        idem: opt_idem(r.idem),
                                                                                        name: r.name,
                                                                                        description: r.description,
                                                                                    })
                                                                                    .collect()
                                                                            );
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelCreate(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbChannelGroupId,
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::{
                                                                                ChannelCreate,
                                                                                ChannelRes,
                                                                            },
                                                                        },
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<ChannelRes, loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (&State, AccountId, ChannelCreate,) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let db_identity = DbIdentity(req.identity);
                                                                        let db_channel_id =
                                                                            DbChannelId(random_channel_id());
                                                                        let idem =
                                                                            req.idem.clone().unwrap_or_default();
                                                                        let group = req.group.map(DbChannelGroupId);
                                                                        let page_size = state.default_page_size as i64;
                                                                        let row = tx(&state.db, move |db_tx| {
                                                                            require_owned_identity(
                                                                                db_tx,
                                                                                account_id,
                                                                                db_identity
                                                                            )?;
                                                                            if !idem.is_empty() {
                                                                                let existing =
                                                                                    good_ormning
                                                                                    ::sqlite
                                                                                    ::good_query_opt!(
                                                                                        db,
                                                                                        r#"select id from channel where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_identity} and idem = ${string = idem.as_str()} and deleted is null"#;
                                                                                        db_tx
                                                                                    ).map_err(|e| loga::err(e.0))?;
                                                                                if let Some(existing) = existing {
                                                                                    return Ok(
                                                                                        channel_row_get(
                                                                                            db_tx,
                                                                                            account_id,
                                                                                            db_identity,
                                                                                            &existing,
                                                                                        )?.context(
                                                                                            "Channel vanished during lookup"
                                                                                        )?,
                                                                                    );
                                                                                }
                                                                            }
                                                                            insert_channel_row(
                                                                                db_tx,
                                                                                account_id,
                                                                                db_identity,
                                                                                &db_channel_id,
                                                                                db_identity,
                                                                                &idem,
                                                                                group,
                                                                                &req.name,
                                                                                &req.description,
                                                                                req.shown_name.as_deref(),
                                                                                req.shown_description.as_deref(),
                                                                                None,
                                                                                page_size,
                                                                                true,
                                                                            )?;
                                                                            return Ok(
                                                                                channel_row_get(
                                                                                    db_tx,
                                                                                    account_id,
                                                                                    db_identity,
                                                                                    &db_channel_id,
                                                                                )?.context(
                                                                                    "Channel vanished after insert"
                                                                                )?,
                                                                            );
                                                                        }).await.context("Error inserting channel")?;
                                                                        return Ok(row.into_res());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelJoinChannel(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            subsystems::pull_scheduler::PullKey,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::{
                                                                                AccountId,
                                                                                ChannelInviteToken,
                                                                            },
                                                                            wire::c2s::{
                                                                                ChannelJoinChannel,
                                                                                ChannelRes,
                                                                            },
                                                                            wire::kwaurl::{
                                                                                KwaUrl,
                                                                                KwaUrlChannelInvite,
                                                                            },
                                                                        },
                                                                        std::sync::Arc,
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<ChannelRes, loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (
                                                                                &Arc<State>,
                                                                                AccountId,
                                                                                ChannelJoinChannel,
                                                                            ) =
                                                                            (state, account, r,);

                                                                        use crate::interface::s2s::{
                                                                            ChannelOrIdentity,
                                                                            InvitationToken,
                                                                            Join,
                                                                        };

                                                                        let account_id = DbAccountId(account);
                                                                        let sender = req.sender;
                                                                        let owner = req.channel.identity;
                                                                        validate_join_name(&req.name)?;
                                                                        let portrait =
                                                                            resolve_join_portrait(
                                                                                state,
                                                                                account_id,
                                                                                &KwaUrl::ChannelInvite(
                                                                                    KwaUrlChannelInvite {
                                                                                        channel: req.channel.clone(),
                                                                                        code: req.code.clone(),
                                                                                    }
                                                                                ),
                                                                                req.portrait
                                                                            ).await?;
                                                                        let signer =
                                                                            crate::subsystems::messages::load_signer(
                                                                                state,
                                                                                account,
                                                                                sender
                                                                            ).await?;
                                                                        let returned =
                                                                            state
                                                                                .s2s_client
                                                                                .join(owner, &signer, Join {
                                                                                    target: ChannelOrIdentity::Channel(
                                                                                        req.channel.channel.clone()
                                                                                    ),
                                                                                    invitation: InvitationToken
                                                                                    ::Channel(
                                                                                        ChannelInviteToken(req.code)
                                                                                    ),
                                                                                })
                                                                                .await
                                                                                .context("s2s Join failed")?;
                                                                        let channel_id = returned.channel;
                                                                        let info = returned.info;
                                                                        let page_size = info.page_size as i64;
                                                                        let row =
                                                                            tx(&state.db, {
                                                                                let channel_id = channel_id.clone();
                                                                                move |db_tx| {
                                                                                    let db_owner = DbIdentity(owner);
                                                                                    let db_channel_id =
                                                                                        DbChannelId(
                                                                                            channel_id.clone()
                                                                                        );
                                                                                    let idem =
                                                                                        format!(
                                                                                            "{}:{}",
                                                                                            owner,
                                                                                            channel_id.0
                                                                                        );
                                                                                    insert_joined_channel(
                                                                                        db_tx,
                                                                                        account_id,
                                                                                        db_owner,
                                                                                        &db_channel_id,
                                                                                        DbIdentity(sender),
                                                                                        &idem,
                                                                                        &req.name,
                                                                                        &req.description,
                                                                                        portrait
                                                                                            .as_ref()
                                                                                            .map(|h| h.0.as_str()),
                                                                                        page_size,
                                                                                        true,
                                                                                    )?;
                                                                                    good_ormning::sqlite::good_query!(
                                                                                        db,
                                                                                        r#"insert into contact (account_id, id, name, description, portrait) values (${account_id_t = &account_id}, ${identity_id_t = &db_owner}, ${string = req.name.as_str()}, ${string = req.description.as_str()}, ${opt string = portrait.as_ref().map(|h| h.0.as_str())}) on conflict (account_id, id) do nothing"#;
                                                                                        db_tx
                                                                                    ).map_err(|e| loga::err(e.0))?;
                                                                                    crate
                                                                                    ::subsystems
                                                                                    ::messages
                                                                                    ::store_channel_info(
                                                                                        db_tx,
                                                                                        account_id,
                                                                                        db_owner,
                                                                                        &db_channel_id,
                                                                                        &info,
                                                                                        true,
                                                                                    )?;
                                                                                    return Ok(
                                                                                        channel_row_get(
                                                                                            db_tx,
                                                                                            account_id,
                                                                                            db_owner,
                                                                                            &db_channel_id
                                                                                        )?.context(
                                                                                            "Joined channel vanished"
                                                                                        )?,
                                                                                    );
                                                                                }
                                                                            })
                                                                                .await
                                                                                .context(
                                                                                    "Error inserting joined channel row"
                                                                                )?;
                                                                        state
                                                                            .pull_scheduler
                                                                            .trigger_immediate(PullKey {
                                                                                account_id: account,
                                                                                owner: owner,
                                                                                channel: channel_id,
                                                                                source: owner,
                                                                            })
                                                                            .await;
                                                                        return Ok(row.into_res());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelJoinIdentity(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            subsystems::pull_scheduler::PullKey,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::{
                                                                                AccountId,
                                                                                IdentityInviteToken,
                                                                            },
                                                                            wire::c2s::{
                                                                                ChannelJoinIdentity,
                                                                                ChannelRes,
                                                                            },
                                                                            wire::kwaurl::{
                                                                                KwaUrl,
                                                                                KwaUrlIdentityInvite,
                                                                            },
                                                                        },
                                                                        std::sync::Arc,
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<ChannelRes, loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (
                                                                                &Arc<State>,
                                                                                AccountId,
                                                                                ChannelJoinIdentity,
                                                                            ) =
                                                                            (state, account, r,);

                                                                        use crate::interface::s2s::{
                                                                            ChannelOrIdentity,
                                                                            InvitationToken,
                                                                            Join,
                                                                        };

                                                                        let account_id = DbAccountId(account);
                                                                        let sender = req.sender;
                                                                        let target = req.identity;
                                                                        validate_join_name(&req.name)?;
                                                                        let portrait =
                                                                            resolve_join_portrait(
                                                                                state,
                                                                                account_id,
                                                                                &KwaUrl::IdentityInvite(
                                                                                    KwaUrlIdentityInvite {
                                                                                        identity: target,
                                                                                        code: req.code.clone(),
                                                                                    }
                                                                                ),
                                                                                req.portrait
                                                                            ).await?;
                                                                        let signer =
                                                                            crate::subsystems::messages::load_signer(
                                                                                state,
                                                                                account,
                                                                                sender
                                                                            ).await?;
                                                                        let returned =
                                                                            state
                                                                                .s2s_client
                                                                                .join(target, &signer, Join {
                                                                                    target: ChannelOrIdentity::Identity(
                                                                                        target
                                                                                    ),
                                                                                    invitation: InvitationToken
                                                                                    ::Identity(
                                                                                        IdentityInviteToken(req.code)
                                                                                    ),
                                                                                })
                                                                                .await
                                                                                .context(
                                                                                    "s2s Join (identity) failed"
                                                                                )?;
                                                                        let channel_id = returned.channel;
                                                                        let info = returned.info;
                                                                        let page_size = info.page_size as i64;
                                                                        let row =
                                                                            tx(&state.db, {
                                                                                let channel_id = channel_id.clone();
                                                                                move |db_tx| {
                                                                                    let db_target =
                                                                                        DbIdentity(target);
                                                                                    let db_channel_id =
                                                                                        DbChannelId(
                                                                                            channel_id.clone()
                                                                                        );
                                                                                    good_ormning::sqlite::good_query!(
                                                                                        db,
                                                                                        r#"insert into contact (account_id, id, name, description, portrait) values (${account_id_t = &account_id}, ${identity_id_t = &db_target}, ${string = req.name.as_str()}, ${string = req.description.as_str()}, ${opt string = portrait.as_ref().map(|h| h.0.as_str())}) on conflict (account_id, id) do update set name = excluded.name, description = excluded.description, portrait = excluded.portrait"#;
                                                                                        db_tx
                                                                                    ).map_err(|e| loga::err(e.0))?;
                                                                                    let idem =
                                                                                        format!(
                                                                                            "dm:{}:{}",
                                                                                            target,
                                                                                            channel_id.0
                                                                                        );
                                                                                    insert_joined_channel(
                                                                                        db_tx,
                                                                                        account_id,
                                                                                        db_target,
                                                                                        &db_channel_id,
                                                                                        DbIdentity(sender),
                                                                                        &idem,
                                                                                        &req.name,
                                                                                        &req.description,
                                                                                        portrait
                                                                                            .as_ref()
                                                                                            .map(|h| h.0.as_str()),
                                                                                        page_size,
                                                                                        true,
                                                                                    )?;
                                                                                    crate
                                                                                    ::subsystems
                                                                                    ::messages
                                                                                    ::store_channel_info(
                                                                                        db_tx,
                                                                                        account_id,
                                                                                        db_target,
                                                                                        &db_channel_id,
                                                                                        &info,
                                                                                        true,
                                                                                    )?;
                                                                                    return Ok(
                                                                                        channel_row_get(
                                                                                            db_tx,
                                                                                            account_id,
                                                                                            db_target,
                                                                                            &db_channel_id
                                                                                        )?.context(
                                                                                            "Joined channel vanished"
                                                                                        )?,
                                                                                    );
                                                                                }
                                                                            })
                                                                                .await
                                                                                .context(
                                                                                    "Error inserting DM channel after identity join"
                                                                                )?;
                                                                        state
                                                                            .pull_scheduler
                                                                            .trigger_immediate(PullKey {
                                                                                account_id: account,
                                                                                owner: target,
                                                                                channel: channel_id,
                                                                                source: target,
                                                                            })
                                                                            .await;
                                                                        return Ok(row.into_res());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelAdd(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            subsystems::pull_scheduler::PullKey,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::{
                                                                                ChannelAdd,
                                                                                ChannelRes,
                                                                            },
                                                                            wire::kwaurl::KwaUrl,
                                                                        },
                                                                        std::sync::Arc,
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<ChannelRes, loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (&Arc<State>, AccountId, ChannelAdd,) =
                                                                            (state, account, r,);

                                                                        use crate::interface::s2s::GetChannelInfo;

                                                                        let account_id = DbAccountId(account);
                                                                        let local_identity = req.identity;
                                                                        let owner = req.channel.identity;
                                                                        let channel_id = req.channel.channel.clone();
                                                                        if local_identity == owner {
                                                                            return Err(
                                                                                loga::err(
                                                                                    "Can't add a channel with its owning identity"
                                                                                )
                                                                            );
                                                                        }
                                                                        validate_join_name(&req.name)?;
                                                                        tx(&state.db, move |db_tx| {
                                                                            return require_owned_identity(
                                                                                db_tx,
                                                                                account_id,
                                                                                DbIdentity(local_identity)
                                                                            );
                                                                        }).await?;
                                                                        let portrait =
                                                                            resolve_join_portrait(
                                                                                state,
                                                                                account_id,
                                                                                &KwaUrl::Channel(req.channel.clone()),
                                                                                req.portrait
                                                                            ).await?;
                                                                        let info =
                                                                            state
                                                                                .s2s_client
                                                                                .get_channel_info(
                                                                                    owner,
                                                                                    None,
                                                                                    GetChannelInfo {
                                                                                        channel: req.channel.clone()
                                                                                    }
                                                                                )
                                                                                .await
                                                                                .context("s2s channel info failed")?;
                                                                        let page_size = info.page_size as i64;
                                                                        let row =
                                                                            tx(&state.db, {
                                                                                let channel_id = channel_id.clone();
                                                                                move |db_tx| {
                                                                                    let db_owner = DbIdentity(owner);
                                                                                    let db_channel_id =
                                                                                        DbChannelId(
                                                                                            channel_id.clone()
                                                                                        );
                                                                                    let idem =
                                                                                        format!(
                                                                                            "{}:{}",
                                                                                            owner,
                                                                                            channel_id.0
                                                                                        );
                                                                                    insert_joined_channel(
                                                                                        db_tx,
                                                                                        account_id,
                                                                                        db_owner,
                                                                                        &db_channel_id,
                                                                                        DbIdentity(local_identity),
                                                                                        &idem,
                                                                                        &req.name,
                                                                                        &req.description,
                                                                                        portrait
                                                                                            .as_ref()
                                                                                            .map(|h| h.0.as_str()),
                                                                                        page_size,
                                                                                        false,
                                                                                    )?;
                                                                                    crate
                                                                                    ::subsystems
                                                                                    ::messages
                                                                                    ::store_channel_info(
                                                                                        db_tx,
                                                                                        account_id,
                                                                                        db_owner,
                                                                                        &db_channel_id,
                                                                                        &info,
                                                                                        true,
                                                                                    )?;
                                                                                    return Ok(
                                                                                        channel_row_get(
                                                                                            db_tx,
                                                                                            account_id,
                                                                                            db_owner,
                                                                                            &db_channel_id
                                                                                        )?.context(
                                                                                            "Added channel vanished"
                                                                                        )?,
                                                                                    );
                                                                                }
                                                                            })
                                                                                .await
                                                                                .context(
                                                                                    "Error inserting added channel row"
                                                                                )?;
                                                                        state
                                                                            .pull_scheduler
                                                                            .trigger_immediate(PullKey {
                                                                                account_id: account,
                                                                                owner: owner,
                                                                                channel: channel_id,
                                                                                source: owner,
                                                                            })
                                                                            .await;
                                                                        return Ok(row.into_res());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelJoinPreview(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::State,
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::{
                                                                                ChannelJoinPreview,
                                                                                ChannelJoinPreviewRes,
                                                                            },
                                                                        },
                                                                        std::sync::Arc,
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret:
                                                                        Result<ChannelJoinPreviewRes, loga::Error> =
                                                                        async {
                                                                            let (state, _account, req,):
                                                                                (
                                                                                    &Arc<State>,
                                                                                    AccountId,
                                                                                    ChannelJoinPreview,
                                                                                ) =
                                                                                (state, account, r,);

                                                                            use crate::interface::s2s::GetJoinPreview;

                                                                            let (host, target, _) =
                                                                                join_remote_from_url(&req.url)?;
                                                                            let res =
                                                                                state
                                                                                    .s2s_client
                                                                                    .get_join_preview(
                                                                                        host,
                                                                                        GetJoinPreview {
                                                                                            target: target
                                                                                        }
                                                                                    )
                                                                                    .await
                                                                                    .context(
                                                                                        "s2s join preview failed"
                                                                                    )?;
                                                                            return Ok(ChannelJoinPreviewRes {
                                                                                name: res.name,
                                                                                description: res.description,
                                                                                portrait: res.portrait,
                                                                            });
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelModify(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbChannelGroupId,
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            subsystems::messages::{
                                                                                retention_days_to_secs,
                                                                                RETENTION_MODE_ALL,
                                                                                RETENTION_MODE_DAYS,
                                                                                RETENTION_MODE_NOTHING,
                                                                                RETENTION_MODE_VIEWED,
                                                                            },
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::{
                                                                                ChannelModify,
                                                                                ChannelRetention,
                                                                                ChannelRes,
                                                                            },
                                                                        },
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<ChannelRes, loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (&State, AccountId, ChannelModify,) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let db_owner = DbIdentity(req.id.identity);
                                                                        let db_channel_id =
                                                                            DbChannelId(req.id.channel.clone());
                                                                        let portrait =
                                                                            resolve_modify_portrait(
                                                                                state,
                                                                                account_id,
                                                                                &req.portrait
                                                                            ).await?;
                                                                        let row = tx(&state.db, move |db_tx| {
                                                                            if !matches!(
                                                                                portrait,
                                                                                PortraitChange::Keep
                                                                            ) {
                                                                                let current =
                                                                                    good_ormning
                                                                                    ::sqlite
                                                                                    ::good_query_opt!(
                                                                                        db,
                                                                                        r#"select portrait from channel where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                        db_tx
                                                                                    )
                                                                                        .map_err(|e| loga::err(e.0))?
                                                                                        .flatten();
                                                                                let next = portrait.apply(current);
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update channel set portrait = ${opt string = next.as_deref()} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(v) = &req.name {
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update channel set name = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(v) = &req.description {
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update channel set description = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(m) = &req.shown_name {
                                                                                require_owned_channel(
                                                                                    db_tx,
                                                                                    account_id,
                                                                                    db_owner,
                                                                                    &db_channel_id
                                                                                )?;
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update channel set shown_name = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(m) = &req.shown_description {
                                                                                require_owned_channel(
                                                                                    db_tx,
                                                                                    account_id,
                                                                                    db_owner,
                                                                                    &db_channel_id
                                                                                )?;
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update channel set shown_description = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(m) = &req.group {
                                                                                let g =
                                                                                    m
                                                                                        .value
                                                                                        .clone()
                                                                                        .map(DbChannelGroupId);
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update channel set channel_group = ${opt channelgroup_id_t = g.as_ref()} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(retention) = req.retention {
                                                                                let row =
                                                                                    channel_row_get(
                                                                                        db_tx,
                                                                                        account_id,
                                                                                        db_owner,
                                                                                        &db_channel_id
                                                                                    )?.context("No such channel")?;
                                                                                let is_owner =
                                                                                    row.local_identity == db_owner.0;
                                                                                let (mode, secs) = match retention {
                                                                                    ChannelRetention::Nothing => (
                                                                                        RETENTION_MODE_NOTHING,
                                                                                        None
                                                                                    ),
                                                                                    ChannelRetention::Viewed => (
                                                                                        RETENTION_MODE_VIEWED,
                                                                                        None
                                                                                    ),
                                                                                    ChannelRetention::All => (
                                                                                        RETENTION_MODE_ALL,
                                                                                        None
                                                                                    ),
                                                                                    ChannelRetention::Days(days) => (
                                                                                        RETENTION_MODE_DAYS,
                                                                                        Some(
                                                                                            retention_days_to_secs(
                                                                                                days
                                                                                            )?
                                                                                        )
                                                                                    ),
                                                                                };
                                                                                let backfill_done = match retention {
                                                                                    ChannelRetention::Nothing |
                                                                                    ChannelRetention::Viewed => {
                                                                                        if is_owner {
                                                                                            return Err(
                                                                                                loga::err(
                                                                                                    "Channel owners must keep everything or a number of days"
                                                                                                )
                                                                                            );
                                                                                        }
                                                                                        true
                                                                                    },
                                                                                    ChannelRetention::All |
                                                                                    ChannelRetention::Days(
                                                                                        _
                                                                                    ) => is_owner,
                                                                                };
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update channel set retention_mode = ${string = mode}, retention_secs = ${opt i64 = secs}, backfill_done = ${bool = backfill_done} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(v) = req.public_read {
                                                                                require_owned_channel(
                                                                                    db_tx,
                                                                                    account_id,
                                                                                    db_owner,
                                                                                    &db_channel_id
                                                                                )?;
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update channel set public_read = ${bool = v} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(v) = req.public_post {
                                                                                require_owned_channel(
                                                                                    db_tx,
                                                                                    account_id,
                                                                                    db_owner,
                                                                                    &db_channel_id
                                                                                )?;
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update channel set public_post = ${bool = v} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(v) = req.reminders {
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update channel set reminders = ${bool = v} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(v) = req.reminder_lead_minutes {
                                                                                let v =
                                                                                    crate
                                                                                    ::subsystems
                                                                                    ::messages
                                                                                    ::u64_to_i64(
                                                                                        v
                                                                                    );
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update channel set reminder_lead_minutes = ${i64 = v} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            return Ok(
                                                                                channel_row_get(
                                                                                    db_tx,
                                                                                    account_id,
                                                                                    db_owner,
                                                                                    &db_channel_id
                                                                                )?.context("No such channel")?
                                                                            );
                                                                        }).await.context("Error modifying channel")?;
                                                                        return Ok(row.into_res());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelDelete(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::ChannelDelete,
                                                                        },
                                                                    };

                                                                    let ret: Result<(), loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (&State, AccountId, ChannelDelete,) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let db_owner = DbIdentity(req.id.identity);
                                                                        let db_channel_id =
                                                                            DbChannelId(req.id.channel.clone());
                                                                        let now = jiff::Timestamp::now();
                                                                        tx(&state.db, move |db_tx| {
                                                                            good_ormning::sqlite::good_query!(
                                                                                db,
                                                                                r#"update channel set deleted = ${time_ms_t = &crate::interface::db::DbTimeMs(now)} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                db_tx
                                                                            ).map_err(|e| loga::err(e.0))?;
                                                                            return Ok(());
                                                                        }).await.context("Error deleting channel")?;
                                                                        {
                                                                            use {
                                                                                shared::interface::shared::{
                                                                                    AccountId,
                                                                                    ChannelId,
                                                                                },
                                                                                spaghettinuum::interface::identity::Identity,
                                                                            };
                                                                            use crate::subsystems::pull_scheduler::*;

                                                                            async {
                                                                                let (
                                                                                    pull_scheduler,
                                                                                    account_id,
                                                                                    owner,
                                                                                    channel,
                                                                                ):
                                                                                    (
                                                                                        &PullScheduler,
                                                                                        AccountId,
                                                                                        Identity,
                                                                                        &ChannelId,
                                                                                    ) =
                                                                                    (
                                                                                        &(state.pull_scheduler),
                                                                                        account,
                                                                                        req.id.identity,
                                                                                        &req.id.channel,
                                                                                    );
                                                                                pull_scheduler
                                                                                    .inner
                                                                                    .lock()
                                                                                    .await
                                                                                    .state
                                                                                    .retain(
                                                                                        |k, _| !(k.account_id ==
                                                                                            account_id &&
                                                                                            k.owner == owner &&
                                                                                            &k.channel == channel)
                                                                                    );
                                                                            }.await
                                                                        };
                                                                        return Ok(());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelList(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::DbAccountId,
                                                                            subsystems::{
                                                                                files::file_ref,
                                                                                messages::retention_from_db,
                                                                            },
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::{
                                                                                ChannelList,
                                                                                ChannelRes,
                                                                            },
                                                                        },
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<Vec<ChannelRes>, loga::Error> =
                                                                        async {
                                                                            let (state, account, _req,):
                                                                                (&State, AccountId, ChannelList,) =
                                                                                (state, account, r,);
                                                                            let account_id = DbAccountId(account);
                                                                            let rows =
                                                                                tx(&state.db, move |db_tx| {
                                                                                    let rows =
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query_many!(
                                                                                            db,
                                                                                            r#"select identity, id, local_identity, idem, channel_group, name, description, shown_name, shown_description, portrait, retention_secs, retention_mode, public_read, public_post, reminders, reminder_lead_minutes from channel where account_id = ${account_id_t = &account_id} and deleted is null order by identity, id"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    let mut out = vec![];
                                                                                    for r in rows {
                                                                                        out.push(ChannelRow {
                                                                                            owner: r.identity.0,
                                                                                            id: r.id.0,
                                                                                            local_identity: r
                                                                                                .local_identity
                                                                                                .0,
                                                                                            idem: r.idem,
                                                                                            group: r
                                                                                                .channel_group
                                                                                                .map(|g| g.0),
                                                                                            name: r.name,
                                                                                            description: r.description,
                                                                                            shown_name: r.shown_name,
                                                                                            shown_description: r.shown_description,
                                                                                            portrait: file_ref(
                                                                                                db_tx,
                                                                                                account_id,
                                                                                                r.portrait
                                                                                            )?,
                                                                                            retention: retention_from_db(
                                                                                                &r.retention_mode,
                                                                                                r.retention_secs
                                                                                            ),
                                                                                            public_read: r.public_read,
                                                                                            public_post: r.public_post,
                                                                                            reminders: r.reminders,
                                                                                            reminder_lead_minutes: r.reminder_lead_minutes,
                                                                                        });
                                                                                    }
                                                                                    return Ok(out);
                                                                                })
                                                                                    .await
                                                                                    .context(
                                                                                        "Error listing channels"
                                                                                    )?;
                                                                            return Ok(
                                                                                rows
                                                                                    .into_iter()
                                                                                    .map(|r| r.into_res())
                                                                                    .collect()
                                                                            );
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelMemberList(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::{
                                                                                ChannelMemberList,
                                                                                ChannelMemberRes,
                                                                            },
                                                                        },
                                                                    };

                                                                    let ret:
                                                                        Result<Vec<ChannelMemberRes>, loga::Error> =
                                                                        async {
                                                                            let (state, account, req,):
                                                                                (
                                                                                    &State,
                                                                                    AccountId,
                                                                                    ChannelMemberList,
                                                                                ) =
                                                                                (state, account, r,);
                                                                            let account_id = DbAccountId(account);
                                                                            let db_owner =
                                                                                DbIdentity(req.channel.identity);
                                                                            let db_channel_id =
                                                                                DbChannelId(req.channel.channel);
                                                                            let mut members =
                                                                                tx(&state.db, move |db_tx| {
                                                                                    return Ok(
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query_many!(
                                                                                            db,
                                                                                            r#"select member, can_post from channel_member where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id}"#;
                                                                                            db_tx
                                                                                        )
                                                                                            .map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?
                                                                                            .into_iter()
                                                                                            .map(|r| ChannelMemberRes {
                                                                                                identity: r.member.0,
                                                                                                can_post: r.can_post,
                                                                                            })
                                                                                            .collect::<Vec<_>>()
                                                                                    );
                                                                                })
                                                                                    .await
                                                                                    .context(
                                                                                        "Error listing channel members"
                                                                                    )?;
                                                                            members.sort_by(
                                                                                |a, b| a.identity.cmp(&b.identity)
                                                                            );
                                                                            return Ok(members);
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelMemberModify(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            subsystems::pull_scheduler::PullKey,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::ChannelMemberModify,
                                                                        },
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<(), loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (&State, AccountId, ChannelMemberModify,) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let db_owner =
                                                                            DbIdentity(req.channel.identity);
                                                                        let db_channel_id =
                                                                            DbChannelId(req.channel.channel.clone());
                                                                        let db_member = DbIdentity(req.member);
                                                                        let changed =
                                                                            tx(&state.db, move |db_tx| {
                                                                                require_owned_channel(
                                                                                    db_tx,
                                                                                    account_id,
                                                                                    db_owner,
                                                                                    &db_channel_id
                                                                                )?;
                                                                                let mut changed = false;
                                                                                if let Some(v) = req.can_post {
                                                                                    good_ormning::sqlite::good_query!(
                                                                                        db,
                                                                                        r#"update channel_member set can_post = ${bool = v} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and member = ${identity_id_t = &db_member}"#;
                                                                                        db_tx
                                                                                    ).map_err(|e| loga::err(e.0))?;
                                                                                    if db_tx.0.changes() == 0 {
                                                                                        return Err(
                                                                                            loga::err(
                                                                                                "No such channel member"
                                                                                            )
                                                                                        );
                                                                                    }
                                                                                    changed = true;
                                                                                }
                                                                                return Ok(changed);
                                                                            })
                                                                                .await
                                                                                .context(
                                                                                    "Error modifying channel member"
                                                                                )?;
                                                                        if changed {
                                                                            let key = PullKey {
                                                                                account_id: account,
                                                                                owner: req.channel.identity,
                                                                                channel: req.channel.channel,
                                                                                source: req.member,
                                                                            };
                                                                            match req.can_post {
                                                                                Some(true) => state
                                                                                    .pull_scheduler
                                                                                    .add_or_reset(key)
                                                                                    .await,
                                                                                Some(false) => state
                                                                                    .pull_scheduler
                                                                                    .remove(&key)
                                                                                    .await,
                                                                                None => { },
                                                                            }
                                                                        }
                                                                        return Ok(());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelMemberDelete(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            subsystems::pull_scheduler::PullKey,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::ChannelMemberDelete,
                                                                        },
                                                                    };

                                                                    let ret: Result<(), loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (&State, AccountId, ChannelMemberDelete,) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let db_owner =
                                                                            DbIdentity(req.channel.identity);
                                                                        let db_channel_id =
                                                                            DbChannelId(req.channel.channel.clone());
                                                                        let db_member = DbIdentity(req.member);
                                                                        tx(&state.db, move |db_tx| {
                                                                            good_ormning::sqlite::good_query!(
                                                                                db,
                                                                                r#"delete from channel_member where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and member = ${identity_id_t = &db_member}"#;
                                                                                db_tx
                                                                            ).map_err(|e| loga::err(e.0))?;
                                                                            return Ok(());
                                                                        })
                                                                            .await
                                                                            .context("Error deleting channel member")?;
                                                                        state.pull_scheduler.remove(&PullKey {
                                                                            account_id: account,
                                                                            owner: req.channel.identity,
                                                                            channel: req.channel.channel,
                                                                            source: req.member,
                                                                        }).await;
                                                                        return Ok(());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelInviteCreate(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            subsystems::messages::i64_to_u64,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::{
                                                                                AccountId,
                                                                                ChannelInviteId,
                                                                                ChannelInviteToken,
                                                                                QualifiedChannelInviteToken,
                                                                            },
                                                                            wire::c2s::{
                                                                                ChannelInviteCreate,
                                                                                ChannelInviteRes,
                                                                            },
                                                                        },
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<ChannelInviteRes, loga::Error> =
                                                                        async {
                                                                            let (state, account, req,):
                                                                                (
                                                                                    &State,
                                                                                    AccountId,
                                                                                    ChannelInviteCreate,
                                                                                ) =
                                                                                (state, account, r,);
                                                                            let account_id = DbAccountId(account);
                                                                            let channel = req.channel.clone();
                                                                            let db_owner =
                                                                                DbIdentity(channel.identity);
                                                                            let db_channel_id =
                                                                                DbChannelId(channel.channel.clone());
                                                                            let idem =
                                                                                req.idem.clone().unwrap_or_default();
                                                                            let token = random_token();
                                                                            return Ok(
                                                                                tx(&state.db, move |db_tx| {
                                                                                    require_owned_channel(
                                                                                        db_tx,
                                                                                        account_id,
                                                                                        db_owner,
                                                                                        &db_channel_id
                                                                                    )?;
                                                                                    if !idem.is_empty() {
                                                                                        let existing =
                                                                                            good_ormning
                                                                                            ::sqlite
                                                                                            ::good_query_opt!(
                                                                                                db,
                                                                                                r#"select rowid, token, name, description, shown_name, shown_description, single_use, expiry, can_post from channel_invite where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and idem = ${string = idem.as_str()}"#;
                                                                                                db_tx
                                                                                            ).map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?;
                                                                                        if let Some(e) = existing {
                                                                                            return Ok(
                                                                                                ChannelInviteRes {
                                                                                                    id: ChannelInviteId(
                                                                                                        i64_to_u64(
                                                                                                            e.rowid
                                                                                                        )
                                                                                                    ),
                                                                                                    token: QualifiedChannelInviteToken {
                                                                                                        channel: channel,
                                                                                                        token: ChannelInviteToken(
                                                                                                            e.token
                                                                                                        ),
                                                                                                    },
                                                                                                    name: e.name,
                                                                                                    description: e.description,
                                                                                                    shown_name: e.shown_name,
                                                                                                    shown_description: e.shown_description,
                                                                                                    single_use: e.single_use,
                                                                                                    expiry: e
                                                                                                        .expiry
                                                                                                        .map(|t| t.0),
                                                                                                    can_post: e.can_post,
                                                                                                }
                                                                                            );
                                                                                        }
                                                                                    }
                                                                                    good_ormning::sqlite::good_query!(
                                                                                        db,
                                                                                        r#"insert into channel_invite (account_id, identity, channel, idem, token, name, description, shown_name, shown_description, single_use, expiry, used_count, can_post) values (${account_id_t = &account_id}, ${identity_id_t = &db_owner}, ${channel_id_t = &db_channel_id}, ${string = idem.as_str()}, ${string = token.as_str()}, ${string = req.name.as_str()}, ${string = req.description.as_str()}, ${opt string = req.shown_name.as_deref()}, ${opt string = req.shown_description.as_deref()}, ${bool = req.single_use}, ${opt time_ms_t = req.expiry.map(crate::interface::db::DbTimeMs).as_ref()}, ${i64 = 0}, ${bool = req.can_post})"#;
                                                                                        db_tx
                                                                                    ).map_err(|e| loga::err(e.0))?;
                                                                                    return Ok(ChannelInviteRes {
                                                                                        id: ChannelInviteId(
                                                                                            i64_to_u64(
                                                                                                db_tx
                                                                                                    .0
                                                                                                    .last_insert_rowid()
                                                                                            )
                                                                                        ),
                                                                                        token: QualifiedChannelInviteToken {
                                                                                            channel: channel,
                                                                                            token: ChannelInviteToken(
                                                                                                token
                                                                                            ),
                                                                                        },
                                                                                        name: req.name,
                                                                                        description: req.description,
                                                                                        shown_name: req.shown_name,
                                                                                        shown_description: req.shown_description,
                                                                                        single_use: req.single_use,
                                                                                        expiry: req.expiry,
                                                                                        can_post: req.can_post,
                                                                                    });
                                                                                })
                                                                                    .await
                                                                                    .context(
                                                                                        "Error inserting channel invite"
                                                                                    )?
                                                                            );
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelInviteModify(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::DbAccountId,
                                                                            subsystems::messages::i64_to_u64,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::{
                                                                                AccountId,
                                                                                ChannelInviteId,
                                                                                ChannelInviteToken,
                                                                                QualifiedChannelId,
                                                                                QualifiedChannelInviteToken,
                                                                            },
                                                                            wire::c2s::{
                                                                                ChannelInviteModify,
                                                                                ChannelInviteRes,
                                                                            },
                                                                        },
                                                                    };

                                                                    let ret: Result<ChannelInviteRes, loga::Error> =
                                                                        async {
                                                                            let (state, account, req,):
                                                                                (
                                                                                    &State,
                                                                                    AccountId,
                                                                                    ChannelInviteModify,
                                                                                ) =
                                                                                (state, account, r,);
                                                                            let account_id = DbAccountId(account);
                                                                            let rowid =
                                                                                crate
                                                                                ::subsystems
                                                                                ::messages
                                                                                ::u64_to_i64(
                                                                                    req.id.0
                                                                                );
                                                                            let row =
                                                                                tx(&state.db, move |db_tx| {
                                                                                    if let Some(v) = &req.name {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update channel_invite set name = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    if let Some(v) = &req.description {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update channel_invite set description = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    if let Some(m) = &req.shown_name {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update channel_invite set shown_name = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    if let Some(m) =
                                                                                        &req.shown_description {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update channel_invite set shown_description = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    if let Some(v) = req.single_use {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update channel_invite set single_use = ${bool = v} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    if let Some(m) = req.expiry {
                                                                                        let v = m.value;
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update channel_invite set expiry = ${opt time_ms_t = v.map(crate::interface::db::DbTimeMs).as_ref()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    if let Some(v) = req.can_post {
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query!(
                                                                                            db,
                                                                                            r#"update channel_invite set can_post = ${bool = v} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    }
                                                                                    return Ok(
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query_opt!(
                                                                                            db,
                                                                                            r#"select rowid, identity, channel, token, name, description, shown_name, shown_description, single_use, expiry, can_post from channel_invite where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                            db_tx
                                                                                        )
                                                                                            .map_err(
                                                                                                |e| loga::err(e.0)
                                                                                            )?
                                                                                            .context(
                                                                                                "No such channel invite"
                                                                                            )?
                                                                                    );
                                                                                })
                                                                                    .await
                                                                                    .context(
                                                                                        "Error modifying channel invite"
                                                                                    )?;
                                                                            return Ok(ChannelInviteRes {
                                                                                id: ChannelInviteId(
                                                                                    i64_to_u64(row.rowid)
                                                                                ),
                                                                                token: QualifiedChannelInviteToken {
                                                                                    channel: QualifiedChannelId {
                                                                                        identity: row.identity.0,
                                                                                        channel: row.channel.0,
                                                                                    },
                                                                                    token: ChannelInviteToken(
                                                                                        row.token
                                                                                    ),
                                                                                },
                                                                                name: row.name,
                                                                                description: row.description,
                                                                                shown_name: row.shown_name,
                                                                                shown_description: row.shown_description,
                                                                                single_use: row.single_use,
                                                                                expiry: row.expiry.map(|t| t.0),
                                                                                can_post: row.can_post,
                                                                            });
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelInviteDelete(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::DbAccountId,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::ChannelInviteDelete,
                                                                        },
                                                                    };

                                                                    let ret: Result<(), loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (&State, AccountId, ChannelInviteDelete,) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let rowid =
                                                                            crate::subsystems::messages::u64_to_i64(
                                                                                req.id.0
                                                                            );
                                                                        tx(&state.db, move |db_tx| {
                                                                            good_ormning::sqlite::good_query!(
                                                                                db,
                                                                                r#"delete from channel_invite where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                                                                                db_tx
                                                                            ).map_err(|e| loga::err(e.0))?;
                                                                            return Ok(());
                                                                        })
                                                                            .await
                                                                            .context("Error deleting channel invite")?;
                                                                        return Ok(());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ChannelInviteList(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::DbAccountId,
                                                                            subsystems::messages::i64_to_u64,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::{
                                                                                AccountId,
                                                                                ChannelInviteId,
                                                                                ChannelInviteToken,
                                                                                QualifiedChannelId,
                                                                                QualifiedChannelInviteToken,
                                                                            },
                                                                            wire::c2s::{
                                                                                ChannelInviteList,
                                                                                ChannelInviteRes,
                                                                            },
                                                                        },
                                                                    };

                                                                    let ret:
                                                                        Result<Vec<ChannelInviteRes>, loga::Error> =
                                                                        async {
                                                                            let (state, account, _req,):
                                                                                (
                                                                                    &State,
                                                                                    AccountId,
                                                                                    ChannelInviteList,
                                                                                ) =
                                                                                (state, account, r,);
                                                                            let account_id = DbAccountId(account);
                                                                            let rows =
                                                                                tx(&state.db, move |db_tx| {
                                                                                    return Ok(
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query_many!(
                                                                                            db,
                                                                                            r#"select rowid, identity, channel, token, name, description, shown_name, shown_description, single_use, expiry, can_post from channel_invite where account_id = ${account_id_t = &account_id} order by rowid"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?
                                                                                    );
                                                                                })
                                                                                    .await
                                                                                    .context(
                                                                                        "Error listing channel invites"
                                                                                    )?;
                                                                            return Ok(
                                                                                rows
                                                                                    .into_iter()
                                                                                    .map(|r| ChannelInviteRes {
                                                                                        id: ChannelInviteId(
                                                                                            i64_to_u64(r.rowid)
                                                                                        ),
                                                                                        token: QualifiedChannelInviteToken {
                                                                                            channel: QualifiedChannelId {
                                                                                                identity: r.identity.0,
                                                                                                channel: r.channel.0,
                                                                                            },
                                                                                            token: ChannelInviteToken(
                                                                                                r.token
                                                                                            ),
                                                                                        },
                                                                                        name: r.name,
                                                                                        description: r.description,
                                                                                        shown_name: r.shown_name,
                                                                                        shown_description: r.shown_description,
                                                                                        single_use: r.single_use,
                                                                                        expiry: r.expiry.map(|t| t.0),
                                                                                        can_post: r.can_post,
                                                                                    })
                                                                                    .collect()
                                                                            );
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ContactList(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::DbAccountId,
                                                                            subsystems::files::file_ref,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::{
                                                                                ContactList,
                                                                                ContactRes,
                                                                            },
                                                                        },
                                                                    };

                                                                    let ret: Result<Vec<ContactRes>, loga::Error> =
                                                                        async {
                                                                            let (state, account, _req,):
                                                                                (&State, AccountId, ContactList,) =
                                                                                (state, account, r,);
                                                                            let account_id = DbAccountId(account);
                                                                            return Ok(
                                                                                tx(&state.db, move |db_tx| {
                                                                                    let rows =
                                                                                        good_ormning
                                                                                        ::sqlite
                                                                                        ::good_query_many!(
                                                                                            db,
                                                                                            r#"select id, name, description, portrait from contact where account_id = ${account_id_t = &account_id} order by id"#;
                                                                                            db_tx
                                                                                        ).map_err(|e| loga::err(e.0))?;
                                                                                    let mut out = vec![];
                                                                                    for r in rows {
                                                                                        out.push(ContactRes {
                                                                                            id: r.id.0,
                                                                                            name: r.name,
                                                                                            description: r.description,
                                                                                            portrait: file_ref(
                                                                                                db_tx,
                                                                                                account_id,
                                                                                                r.portrait
                                                                                            )?,
                                                                                        });
                                                                                    }
                                                                                    return Ok(out);
                                                                                })
                                                                                    .await
                                                                                    .context("Error listing contacts")?
                                                                            );
                                                                        }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::ContactModify(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbIdentity,
                                                                            },
                                                                            subsystems::files::file_ref,
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::AccountId,
                                                                            wire::c2s::{
                                                                                ContactModify,
                                                                                ContactRes,
                                                                            },
                                                                        },
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<ContactRes, loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (&State, AccountId, ContactModify,) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let db_id = DbIdentity(req.id);
                                                                        let portrait =
                                                                            resolve_modify_portrait(
                                                                                state,
                                                                                account_id,
                                                                                &req.portrait
                                                                            ).await?;
                                                                        let row = tx(&state.db, move |db_tx| {
                                                                            good_ormning::sqlite::good_query!(
                                                                                db,
                                                                                r#"insert into contact (account_id, id, name, description) values (${account_id_t = &account_id}, ${identity_id_t = &db_id}, ${string = ""}, ${string = ""}) on conflict do nothing"#;
                                                                                db_tx
                                                                            ).map_err(|e| loga::err(e.0))?;
                                                                            if let Some(v) = &req.name {
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update contact set name = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if let Some(v) = &req.description {
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update contact set description = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            if !matches!(
                                                                                portrait,
                                                                                PortraitChange::Keep
                                                                            ) {
                                                                                let current =
                                                                                    good_ormning
                                                                                    ::sqlite
                                                                                    ::good_query_opt!(
                                                                                        db,
                                                                                        r#"select portrait from contact where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_id}"#;
                                                                                        db_tx
                                                                                    )
                                                                                        .map_err(|e| loga::err(e.0))?
                                                                                        .flatten();
                                                                                let next = portrait.apply(current);
                                                                                good_ormning::sqlite::good_query!(
                                                                                    db,
                                                                                    r#"update contact set portrait = ${opt string = next.as_deref()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_id}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            }
                                                                            let row =
                                                                                good_ormning::sqlite::good_query_opt!(
                                                                                    db,
                                                                                    r#"select id, name, description, portrait from contact where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_id}"#;
                                                                                    db_tx
                                                                                )
                                                                                    .map_err(|e| loga::err(e.0))?
                                                                                    .context(
                                                                                        "Contact insertion failed"
                                                                                    )?;
                                                                            return Ok(ContactRes {
                                                                                id: row.id.0,
                                                                                name: row.name,
                                                                                description: row.description,
                                                                                portrait: file_ref(
                                                                                    db_tx,
                                                                                    account_id,
                                                                                    row.portrait
                                                                                )?,
                                                                            });
                                                                        }).await.context("Error modifying contact")?;
                                                                        return Ok(row);
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                            c2s::proto::ServerReq::MessagePush(_, r) => {
                                                                return api_resp(state, {
                                                                    use {
                                                                        crate::{
                                                                            db,
                                                                            dbutil::tx,
                                                                            interface::db::{
                                                                                DbAccountId,
                                                                                DbChannelId,
                                                                                DbIdentity,
                                                                            },
                                                                            subsystems::messages::{
                                                                                channel_members,
                                                                                i64_to_u64,
                                                                                ingest_message,
                                                                                load_signer_tx,
                                                                                message_store,
                                                                                StoredMessage,
                                                                                notify_remotes,
                                                                                publish_local_notification,
                                                                            },
                                                                            State,
                                                                        },
                                                                        loga::ResultContext,
                                                                        shared::interface::{
                                                                            shared::{
                                                                                AccountId,
                                                                                Message,
                                                                                MessageBody,
                                                                                MessageId,
                                                                            },
                                                                            wire::c2s::MessagePush,
                                                                        },
                                                                        spaghettinuum::{
                                                                            byteszb32::BytesZb32,
                                                                            jsonsig::JsonSignature,
                                                                            sigdomain::SigDomained,
                                                                        },
                                                                        spaghettinuum_native::utils::identity_secret::IdentitySigner,
                                                                        std::sync::Arc,
                                                                    };
                                                                    use crate::subsystems::c2s::*;

                                                                    let ret: Result<(), loga::Error> = async {
                                                                        let (state, account, req,):
                                                                            (&Arc<State>, AccountId, MessagePush,) =
                                                                            (state, account, r,);
                                                                        let account_id = DbAccountId(account);
                                                                        let sender = req.identity;
                                                                        let owner = req.channel.identity;
                                                                        let channel = req.channel.clone();
                                                                        let blocks = req.blocks.clone();
                                                                        let max_file_size = state.max_file_size;
                                                                        let result = tx(&state.db, move |db_tx| {
                                                                            let db_sender = DbIdentity(sender);
                                                                            let db_owner = DbIdentity(owner);
                                                                            let db_channel_id =
                                                                                DbChannelId(
                                                                                    req.channel.channel.clone()
                                                                                );
                                                                            let channel_row =
                                                                                good_ormning::sqlite::good_query_opt!(
                                                                                    db,
                                                                                    r#"select local_identity, next_sender_unique from channel where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id} and deleted is null"#;
                                                                                    db_tx
                                                                                )
                                                                                    .map_err(|e| loga::err(e.0))?
                                                                                    .context("No such channel")?;
                                                                            if channel_row.local_identity.0 != sender {
                                                                                return Err(
                                                                                    loga::err(
                                                                                        "Sender isn't this account's identity in the channel"
                                                                                    )
                                                                                );
                                                                            }
                                                                            for block in &req.blocks {
                                                                                if let shared
                                                                                ::interface
                                                                                ::shared
                                                                                ::MessageBlock
                                                                                ::Event(
                                                                                    event
                                                                                ) =
                                                                                    block {
                                                                                    shared::calendar::validate_event(
                                                                                        event
                                                                                    ).map_err(
                                                                                        |e| loga::err_with(
                                                                                            "Message contains an invalid event",
                                                                                            loga::ea!(err = e)
                                                                                        )
                                                                                    )?;
                                                                                }
                                                                            }
                                                                            for block in &req.blocks {
                                                                                let shared
                                                                                ::interface
                                                                                ::shared
                                                                                ::MessageBlock
                                                                                ::File(
                                                                                    file
                                                                                ) =
                                                                                    block else {
                                                                                        continue;
                                                                                    };
                                                                                if file.file.identity != owner {
                                                                                    return Err(
                                                                                        loga::err(
                                                                                            "Message files must be addressed to the channel owner"
                                                                                        )
                                                                                    );
                                                                                }
                                                                                if file.size > max_file_size {
                                                                                    return Err(
                                                                                        loga::err(
                                                                                            "Message file exceeds the size limit"
                                                                                        )
                                                                                    );
                                                                                }
                                                                                let hash = file.file.hash.0.clone();
                                                                                if crate::subsystems::files::file_ref(
                                                                                    db_tx,
                                                                                    account_id,
                                                                                    Some(hash.clone())
                                                                                )?.is_some() {
                                                                                    continue;
                                                                                }
                                                                                let indexed =
                                                                                    good_ormning
                                                                                    ::sqlite
                                                                                    ::good_query_opt!(
                                                                                        db,
                                                                                        r#"select 1 as x from message_file where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and hash = ${string = hash.as_str()} limit ${i64 = 1}"#;
                                                                                        db_tx
                                                                                    ).map_err(|e| loga::err(e.0))?;
                                                                                if indexed.is_none() {
                                                                                    return Err(
                                                                                        loga::err(
                                                                                            "Message references a file that hasn't been uploaded"
                                                                                        )
                                                                                    );
                                                                                }
                                                                            }
                                                                            let existing =
                                                                                good_ormning::sqlite::good_query_opt!(
                                                                                    db,
                                                                                    r#"select seq from message where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and sender = ${identity_id_t = &db_sender} and client_id = ${string = req.client_id.0.as_str()}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            if existing.is_some() {
                                                                                return Ok(None);
                                                                            }
                                                                            let existing_outbox =
                                                                                good_ormning::sqlite::good_query_opt!(
                                                                                    db,
                                                                                    r#"select sender_unique from outbox where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and sender = ${identity_id_t = &db_sender} and client_id = ${string = req.client_id.0.as_str()}"#;
                                                                                    db_tx
                                                                                ).map_err(|e| loga::err(e.0))?;
                                                                            if existing_outbox.is_some() {
                                                                                return Ok(None);
                                                                            }
                                                                            let mut signer =
                                                                                load_signer_tx(
                                                                                    db_tx,
                                                                                    account_id,
                                                                                    db_sender
                                                                                )?;
                                                                            let sender_unique =
                                                                                channel_row.next_sender_unique;
                                                                            let next_sender_unique =
                                                                                sender_unique
                                                                                    .checked_add(1)
                                                                                    .context(
                                                                                        "sender_unique counter overflow"
                                                                                    )?;
                                                                            good_ormning::sqlite::good_query!(
                                                                                db,
                                                                                r#"update channel set next_sender_unique = ${i64 = next_sender_unique} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                                                                                db_tx
                                                                            ).map_err(|e| loga::err(e.0))?;
                                                                            let body = MessageBody {
                                                                                channel: req.channel.clone(),
                                                                                client_id: Some(req.client_id.clone()),
                                                                                id: MessageId {
                                                                                    identity: sender,
                                                                                    unique: i64_to_u64(sender_unique),
                                                                                },
                                                                                rel: req.rel.clone(),
                                                                                blocks: req.blocks.clone(),
                                                                            };
                                                                            let body_json =
                                                                                serde_json::to_string(&body).unwrap();
                                                                            let (_root, _chain, sig) =
                                                                                signer
                                                                                    .sign(
                                                                                        MessageBody::DOMAIN,
                                                                                        body_json.as_bytes()
                                                                                    )
                                                                                    .context(
                                                                                        "Signing MessageBody failed"
                                                                                    )?;
                                                                            let raw =
                                                                                serde_json::to_string(
                                                                                    &Message(JsonSignature {
                                                                                        _p: Default::default(),
                                                                                        message: body_json,
                                                                                        signature: BytesZb32(sig),
                                                                                    })
                                                                                ).unwrap();
                                                                            if sender == owner {
                                                                                let seq =
                                                                                    ingest_message(
                                                                                        db_tx,
                                                                                        account_id,
                                                                                        db_owner,
                                                                                        &db_channel_id,
                                                                                        &raw,
                                                                                        &body,
                                                                                    )?.context(
                                                                                        "Message conflicted with an existing message"
                                                                                    )?;
                                                                                return Ok(Some(PushResult {
                                                                                    seq: Some(seq),
                                                                                    recipients: channel_members(
                                                                                        db_tx,
                                                                                        account_id,
                                                                                        db_owner,
                                                                                        &db_channel_id
                                                                                    )?,
                                                                                }));
                                                                            } else {
                                                                                if !message_store(
                                                                                    db_tx,
                                                                                    account_id,
                                                                                    db_owner,
                                                                                    &db_channel_id,
                                                                                    &raw,
                                                                                    &body,
                                                                                    StoredMessage::Outbox {
                                                                                        created: jiff::Timestamp::now()
                                                                                    }
                                                                                )? {
                                                                                    return Err(
                                                                                        loga::err(
                                                                                            "Message conflicted with an existing outbox message"
                                                                                        )
                                                                                    );
                                                                                }
                                                                                return Ok(Some(PushResult {
                                                                                    seq: None,
                                                                                    recipients: vec![owner],
                                                                                }));
                                                                            }
                                                                        }).await.context("Error storing message")?;
                                                                        let Some(result) = result else {
                                                                            return Ok(());
                                                                        };
                                                                        match result.seq {
                                                                            Some(seq) => {
                                                                                publish_local_notification(
                                                                                    state,
                                                                                    account,
                                                                                    channel.clone(),
                                                                                    seq,
                                                                                    blocks,
                                                                                    false
                                                                                ).await;
                                                                            },
                                                                            None => {
                                                                                state
                                                                                    .ws_bus
                                                                                    .publish(
                                                                                        account,
                                                                                        shared
                                                                                        ::interface
                                                                                        ::wire
                                                                                        ::s2c
                                                                                        ::WsEvent
                                                                                        ::Outbox(
                                                                                            shared
                                                                                            ::interface
                                                                                            ::wire
                                                                                            ::s2c
                                                                                            ::OutboxChange {
                                                                                                channel: channel.clone(),
                                                                                                sender: sender,
                                                                                            }
                                                                                        ),
                                                                                    )
                                                                                    .await;
                                                                            },
                                                                        }
                                                                        notify_remotes(
                                                                            state,
                                                                            account,
                                                                            sender,
                                                                            channel,
                                                                            result.recipients
                                                                        ).await;
                                                                        return Ok(());
                                                                    }.await;
                                                                    ret
                                                                });
                                                            },
                                                        }
                                                    }.await);
                                                },
                                                p if {
                                                    let value: &[&str] =
                                                        &[
                                                            "notification_server_key",
                                                            "portrait",
                                                            "file",
                                                            "join_preview_file",
                                                            "activity_latest_all",
                                                            "activity_page",
                                                            "snap_page",
                                                            "snap_by_id",
                                                            "snap_by_client_id",
                                                            "snap_page_containing_time",
                                                            "outbox",
                                                            "message_history",
                                                            "message_id_by_client_id",
                                                        ];
                                                    value
                                                }.contains(&p) => {
                                                    if head.method != Method::GET {
                                                        return Ok(response_404());
                                                    }
                                                    let Some(account) =
                                                        identify_c2s(state, &head.headers).await.err_internal()? else {
                                                            return Ok(response_401());
                                                        };
                                                    return {
                                                        use {
                                                            crate::{
                                                                db,
                                                                dbutil::tx,
                                                                interface::{
                                                                    db::{
                                                                        DbAccountId,
                                                                        DbIdentity,
                                                                    },
                                                                    s2s::SnapLookupBy,
                                                                },
                                                                subsystems::messages::{
                                                                    earliest_snap_offset,
                                                                    i64_to_u64,
                                                                    local_channel_get,
                                                                    message_history_rows,
                                                                    member_fetch_snap_lookup,
                                                                    parse_blocks,
                                                                    member_fetch_snap_page,
                                                                    member_refresh_info,
                                                                    snap_offset_containing_time,
                                                                    snap_page_rows,
                                                                    snap_position_by_client_id,
                                                                    snap_position_by_id,
                                                                    u64_to_i64,
                                                                    MsgRow,
                                                                    SnapPosition,
                                                                    REL_EDIT_OF,
                                                                    REL_REPLY_TO,
                                                                },
                                                                subsystems::s2s_error::{
                                                                    CachePolicy,
                                                                    Cached,
                                                                    SNAP_PAGE_MAX_AGE_SECS,
                                                                },
                                                                State,
                                                            },
                                                            http::{
                                                                header::{
                                                                    CACHE_CONTROL,
                                                                    CONTENT_SECURITY_POLICY,
                                                                    CONTENT_TYPE,
                                                                    X_CONTENT_TYPE_OPTIONS,
                                                                },
                                                                Response,
                                                            },
                                                            htwrap::htserve::{
                                                                responses::{
                                                                    body_full,
                                                                    response_400,
                                                                    response_404,
                                                                    response_503,
                                                                    Body,
                                                                },
                                                                viserr::VisErr,
                                                            },
                                                            loga::ResultContext,
                                                            shared::interface::{
                                                                shared::{
                                                                    AccountId,
                                                                    Message,
                                                                    MessageBody,
                                                                    MessageClientId,
                                                                    MessageId,
                                                                    MessageRel,
                                                                    QualifiedChannelId,
                                                                    QualifiedMessageId,
                                                                },
                                                                wire::c2s::{
                                                                    ActivityLatestAll,
                                                                    ActivityOffset,
                                                                    ActivityOffsetPos,
                                                                    ActivityPage,
                                                                    ActivityPageMessage,
                                                                    ActivityPageOffsetPos,
                                                                    ActivityPageRes,
                                                                    GetActivityPage,
                                                                    GetFile,
                                                                    GetJoinPreviewFile,
                                                                    GetResp,
                                                                    GetSnapPage,
                                                                    MessageHistory,
                                                                    MessageIdByClientId,
                                                                    MessageRevision,
                                                                    NotificationServerKey,
                                                                    OutboxEntry,
                                                                    OutboxList,
                                                                    PathReqTrait,
                                                                    SnapByClientId,
                                                                    SnapById,
                                                                    SnapByRes,
                                                                    SnapMessage,
                                                                    SnapOffset,
                                                                    SnapOffsetPos,
                                                                    SnapPage,
                                                                    SnapPageContainingTime,
                                                                    SnapPageRes,
                                                                },
                                                            },
                                                            std::{
                                                                collections::{
                                                                    BTreeMap,
                                                                    HashMap,
                                                                },
                                                                sync::Arc,
                                                            },
                                                        };
                                                        use crate::subsystems::c2s_get::*;

                                                        let ret: Result<Response<Body>, VisErr<loga::Error>> = async {
                                                            let (state, account, remainder,):
                                                                (&Arc<State>, AccountId, &str,) =
                                                                (state, account, &path,);
                                                            let first =
                                                                remainder
                                                                    .trim_start_matches('/')
                                                                    .split('/')
                                                                    .next()
                                                                    .unwrap()
                                                                    .to_string();
                                                            match first.as_str() {
                                                                "notification_server_key" => {
                                                                    if NotificationServerKey::deserialize_path(
                                                                        remainder
                                                                    ).is_err() {
                                                                        return Ok(
                                                                            response_400("Bad path".to_string())
                                                                        );
                                                                    }
                                                                    return Ok(
                                                                        json_response(
                                                                            &GetResp::Ok(
                                                                                &state.vapid_keys.public_b64url
                                                                            ),
                                                                            CachePolicy::NoStore
                                                                        )
                                                                    );
                                                                },
                                                                "file" => {
                                                                    let req =
                                                                        match GetFile::deserialize_path(remainder) {
                                                                            Ok(r) => r,
                                                                            Err(e) => return Ok(response_400(e)),
                                                                        };
                                                                    let result =
                                                                        match crate::subsystems::files::file_local(
                                                                            state,
                                                                            DbAccountId(account),
                                                                            &req.hash,
                                                                            false
                                                                        ).await {
                                                                            Ok(v) => v,
                                                                            Err(e) => {
                                                                                state
                                                                                    .log
                                                                                    .log_err(
                                                                                        loga::WARN,
                                                                                        e.context(
                                                                                            "Error fetching file"
                                                                                        )
                                                                                    );
                                                                                return Ok(response_503());
                                                                            },
                                                                        };
                                                                    let Some((mime, path)) = result else {
                                                                        return Ok(response_404());
                                                                    };
                                                                    let mut file_headers = http::HeaderMap::new();
                                                                    file_headers.insert(
                                                                        X_CONTENT_TYPE_OPTIONS,
                                                                        http::HeaderValue::from_static("nosniff")
                                                                    );
                                                                    file_headers.insert(
                                                                        CONTENT_SECURITY_POLICY,
                                                                        http::HeaderValue::from_static(
                                                                            "sandbox; default-src 'none'"
                                                                        )
                                                                    );
                                                                    return Ok(
                                                                        htwrap::htserve::responses::response_file(
                                                                            &head.headers,
                                                                            &mime,
                                                                            &path,
                                                                            true,
                                                                            &file_headers
                                                                        )
                                                                            .await
                                                                            .context("Error serving file")
                                                                            .map_err(VisErr::Internal)?,
                                                                    );
                                                                },
                                                                "join_preview_file" => {
                                                                    let req =
                                                                        match GetJoinPreviewFile::deserialize_path(
                                                                            remainder
                                                                        ) {
                                                                            Ok(r) => r,
                                                                            Err(e) => return Ok(response_400(e)),
                                                                        };
                                                                    let (mime, bytes) =
                                                                        match crate
                                                                        ::subsystems
                                                                        ::c2s
                                                                        ::fetch_join_portrait(
                                                                            state,
                                                                            &req.url,
                                                                            req.hash
                                                                        ).await {
                                                                            Ok(v) => v,
                                                                            Err(e) => {
                                                                                state
                                                                                    .log
                                                                                    .log_err(
                                                                                        loga::DEBUG,
                                                                                        e.context(
                                                                                            "Error fetching join preview file"
                                                                                        )
                                                                                    );
                                                                                return Ok(response_404());
                                                                            },
                                                                        };
                                                                    return Ok(
                                                                        Response::builder()
                                                                            .status(200)
                                                                            .header(CONTENT_TYPE, mime)
                                                                            .header(X_CONTENT_TYPE_OPTIONS, "nosniff")
                                                                            .header(
                                                                                CONTENT_SECURITY_POLICY,
                                                                                "sandbox; default-src 'none'"
                                                                            )
                                                                            .header(
                                                                                CACHE_CONTROL,
                                                                                CachePolicy::NoStore.header_value()
                                                                            )
                                                                            .body(body_full(bytes))
                                                                            .unwrap(),
                                                                    );
                                                                },
                                                                "activity_latest_all" => {
                                                                    if ActivityLatestAll::deserialize_path(
                                                                        remainder
                                                                    ).is_err() {
                                                                        return Ok(
                                                                            response_400("Bad path".to_string())
                                                                        );
                                                                    }
                                                                    return Ok(
                                                                        json_or_err(
                                                                            state,
                                                                            "activity_latest_all",
                                                                            no_store({
                                                                                let ret:
                                                                                    Result<
                                                                                        Vec<
                                                                                            (
                                                                                                QualifiedChannelId,
                                                                                                ActivityOffset
                                                                                            )
                                                                                        >,
                                                                                        loga::Error
                                                                                    > =
                                                                                    async {
                                                                                        let (state, account,):
                                                                                            (&Arc<State>, AccountId,) =
                                                                                            (state, account,);
                                                                                        let account_id =
                                                                                            DbAccountId(account);
                                                                                        let rows =
                                                                                            tx(
                                                                                                &state.db,
                                                                                                move |db_tx| {
                                                                                                    return {
                                                                                                        use {
                                                                                                            crate::{
                                                                                                                db,
                                                                                                                interface::db::DbAccountId,
                                                                                                            },
                                                                                                            rusqlite::Transaction,
                                                                                                            shared::interface::shared::QualifiedChannelId,
                                                                                                            std::collections::HashMap,
                                                                                                        };
                                                                                                        use crate::subsystems::messages::*;

                                                                                                        (|| ->
                                                                                                            Result<
                                                                                                                HashMap<
                                                                                                                    QualifiedChannelId,
                                                                                                                    u64
                                                                                                                >,
                                                                                                                loga
                                                                                                                ::Error
                                                                                                            > {
                                                                                                            let (
                                                                                                                db_tx,
                                                                                                                account,
                                                                                                            ):
                                                                                                                (
                                                                                                                    &mut db
                                                                                                                    ::Db<
                                                                                                                        Transaction<
                                                                                                                            '_
                                                                                                                        >
                                                                                                                    >,
                                                                                                                    DbAccountId,
                                                                                                                ) =
                                                                                                                (
                                                                                                                    db_tx,
                                                                                                                    account_id,
                                                                                                                );
                                                                                                            let rows =
                                                                                                                good_ormning
                                                                                                                ::sqlite
                                                                                                                ::good_query_many!(
                                                                                                                    db,
                                                                                                                    r#"select identity, id, latest_seq from channel where account_id = ${account_id_t = &account} and deleted is null and latest_seq is not null"#;
                                                                                                                    db_tx
                                                                                                                ).map_err(
                                                                                                                    |
                                                                                                                        e
                                                                                                                    | loga
                                                                                                                    ::err(
                                                                                                                        e.0
                                                                                                                    )
                                                                                                                )?;
                                                                                                            let mut out =
                                                                                                                HashMap
                                                                                                                ::new();
                                                                                                            for r in rows {
                                                                                                                let Some(
                                                                                                                    seq
                                                                                                                ) =
                                                                                                                    r.latest_seq else {
                                                                                                                        continue;
                                                                                                                    };
                                                                                                                out.insert(
                                                                                                                    QualifiedChannelId {
                                                                                                                        identity: r
                                                                                                                            .identity
                                                                                                                            .0,
                                                                                                                        channel: r
                                                                                                                            .id
                                                                                                                            .0,
                                                                                                                    },
                                                                                                                    i64_to_u64(
                                                                                                                        seq
                                                                                                                    )
                                                                                                                );
                                                                                                            }
                                                                                                            return Ok(
                                                                                                                out
                                                                                                            );
                                                                                                        })()
                                                                                                    };
                                                                                                }
                                                                                            )
                                                                                                .await
                                                                                                .context(
                                                                                                    "Error querying channel activity"
                                                                                                )?;
                                                                                        return Ok(
                                                                                            rows
                                                                                                .into_iter()
                                                                                                .map(
                                                                                                    |(k, v)| (
                                                                                                        k,
                                                                                                        ActivityOffset(
                                                                                                            v as usize
                                                                                                        )
                                                                                                    )
                                                                                                )
                                                                                                .collect()
                                                                                        );
                                                                                    }.await;
                                                                                ret
                                                                            }.map_err(GetErr::Local)),
                                                                        ),
                                                                    );
                                                                },
                                                                "activity_page" => {
                                                                    let req =
                                                                        match GetActivityPage::deserialize_path(
                                                                            remainder
                                                                        ) {
                                                                            Ok(r) => r,
                                                                            Err(e) => return Ok(response_400(e)),
                                                                        };
                                                                    return Ok(json_or_err(state, "activity_page", {
                                                                        let ret:
                                                                            Result<
                                                                                Cached<Option<ActivityPageRes>>,
                                                                                GetErr
                                                                            > =
                                                                            async {
                                                                                let (state, account, channel, page,):
                                                                                    (
                                                                                        &Arc<State>,
                                                                                        AccountId,
                                                                                        QualifiedChannelId,
                                                                                        ActivityPage,
                                                                                    ) =
                                                                                    (
                                                                                        state,
                                                                                        account,
                                                                                        req.channel,
                                                                                        req.page,
                                                                                    );
                                                                                let ctx =
                                                                                    ChannelCtx::new(account, channel);
                                                                                let Some((local, rows)) =
                                                                                    tx(&state.db, {
                                                                                        let account = ctx.account;
                                                                                        let db_owner = ctx.db_owner;
                                                                                        let db_channel_id =
                                                                                            ctx.db_channel_id.clone();
                                                                                        move |db_tx| {
                                                                                            let Some(local) =
                                                                                                local_channel_get(
                                                                                                    db_tx,
                                                                                                    account,
                                                                                                    db_owner,
                                                                                                    &db_channel_id
                                                                                                )? else {
                                                                                                    return Ok(None);
                                                                                                };
                                                                                            let start =
                                                                                                u64_to_i64(
                                                                                                    page.0 as u64 *
                                                                                                        local.page_size as
                                                                                                            u64
                                                                                                );
                                                                                            let end =
                                                                                                start +
                                                                                                    local.page_size as
                                                                                                        i64;
                                                                                            let rows = {
                                                                                                use {
                                                                                                    crate::{
                                                                                                        db,
                                                                                                        interface::db::{
                                                                                                            DbAccountId,
                                                                                                            DbChannelId,
                                                                                                            DbIdentity,
                                                                                                        },
                                                                                                    },
                                                                                                    rusqlite::Transaction,
                                                                                                };

                                                                                                (|| ->
                                                                                                    Result<
                                                                                                        Vec<
                                                                                                            (
                                                                                                                i64,
                                                                                                                String
                                                                                                            )
                                                                                                        >,
                                                                                                        loga::Error
                                                                                                    > {
                                                                                                    let (
                                                                                                        db_tx,
                                                                                                        account,
                                                                                                        owner,
                                                                                                        channel,
                                                                                                        start,
                                                                                                        end,
                                                                                                    ):
                                                                                                        (
                                                                                                            &mut db
                                                                                                            ::Db<
                                                                                                                Transaction<
                                                                                                                    '_
                                                                                                                >
                                                                                                            >,
                                                                                                            DbAccountId,
                                                                                                            DbIdentity,
                                                                                                            &DbChannelId,
                                                                                                            i64,
                                                                                                            i64,
                                                                                                        ) =
                                                                                                        (
                                                                                                            db_tx,
                                                                                                            account,
                                                                                                            db_owner,
                                                                                                            &db_channel_id,
                                                                                                            start,
                                                                                                            end,
                                                                                                        );
                                                                                                    return Ok(
                                                                                                        good_ormning
                                                                                                        ::sqlite
                                                                                                        ::good_query_many!(
                                                                                                            db,
                                                                                                            r#"select seq, raw from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and seq >= ${i64 = start} and seq < ${i64 = end} order by seq"#;
                                                                                                            db_tx
                                                                                                        )
                                                                                                            .map_err(
                                                                                                                |e| loga
                                                                                                                ::err(
                                                                                                                    e.0
                                                                                                                )
                                                                                                            )?
                                                                                                            .into_iter()
                                                                                                            .map(
                                                                                                                |r| (
                                                                                                                    r.seq,
                                                                                                                    r.raw
                                                                                                                )
                                                                                                            )
                                                                                                            .collect()
                                                                                                    );
                                                                                                })()
                                                                                            }?;
                                                                                            return Ok(
                                                                                                Some((local, rows))
                                                                                            );
                                                                                        }
                                                                                    })
                                                                                        .await
                                                                                        .context(
                                                                                            "Error querying activity page"
                                                                                        )? else {
                                                                                        return Ok(
                                                                                            Cached::no_store(None)
                                                                                        );
                                                                                    };
                                                                                let page_size = local.page_size;
                                                                                let start =
                                                                                    u64_to_i64(
                                                                                        page.0 as u64 * page_size as u64
                                                                                    );
                                                                                let mut merged: BTreeMap<i64, String> =
                                                                                    rows.into_iter().collect();
                                                                                if !local.is_owner(
                                                                                    ctx.channel.identity
                                                                                ) &&
                                                                                    !range_complete(
                                                                                        merged.len(),
                                                                                        start,
                                                                                        page_size,
                                                                                        local.latest_seq,
                                                                                        None
                                                                                    ) {
                                                                                    let fetched = remote({
                                                                                        use {
                                                                                            crate::{
                                                                                                interface::s2s::GetActivity,
                                                                                                State,
                                                                                            },
                                                                                            loga::ResultContext,
                                                                                            shared::interface::shared::{
                                                                                                AccountId,
                                                                                                QualifiedChannelId,
                                                                                            },
                                                                                            std::sync::Arc,
                                                                                        };
                                                                                        use crate::subsystems::messages::*;

                                                                                        let ret:
                                                                                            Result<
                                                                                                Vec<Verified>,
                                                                                                loga::Error
                                                                                            > =
                                                                                            async {
                                                                                                let (
                                                                                                    state,
                                                                                                    account_id,
                                                                                                    channel,
                                                                                                    local,
                                                                                                    start,
                                                                                                    end,
                                                                                                ):
                                                                                                    (
                                                                                                        &Arc<State>,
                                                                                                        AccountId,
                                                                                                        &QualifiedChannelId,
                                                                                                        &LocalChannel,
                                                                                                        u64,
                                                                                                        u64,
                                                                                                    ) =
                                                                                                    (
                                                                                                        state,
                                                                                                        account,
                                                                                                        &ctx.channel,
                                                                                                        &local,
                                                                                                        i64_to_u64(
                                                                                                            start
                                                                                                        ),
                                                                                                        i64_to_u64(
                                                                                                            start
                                                                                                        ) +
                                                                                                            page_size as
                                                                                                                u64,
                                                                                                    );
                                                                                                let signer =
                                                                                                    channel_signer(
                                                                                                        state,
                                                                                                        account_id,
                                                                                                        local
                                                                                                    ).await?;
                                                                                                let mut next = start;
                                                                                                let mut messages =
                                                                                                    vec![];
                                                                                                loop {
                                                                                                    let limit =
                                                                                                        u32::try_from(
                                                                                                            end - next
                                                                                                        )
                                                                                                            .unwrap_or(
                                                                                                                u32::MAX
                                                                                                            )
                                                                                                            .min(
                                                                                                                PULL_BATCH_LIMIT
                                                                                                            );
                                                                                                    let res =
                                                                                                        state
                                                                                                            .s2s_client
                                                                                                            .get_activity(
                                                                                                                channel.identity,
                                                                                                                signer.as_ref(),
                                                                                                                GetActivity {
                                                                                                                    channel: channel.clone(),
                                                                                                                    start: next,
                                                                                                                    limit: limit,
                                                                                                                }
                                                                                                            )
                                                                                                            .await
                                                                                                            .context(
                                                                                                                "Error fetching activity range from channel owner"
                                                                                                            )?;
                                                                                                    let batch_len =
                                                                                                        res
                                                                                                            .messages
                                                                                                            .len();
                                                                                                    let Some(
                                                                                                        batch_max
                                                                                                    ) =
                                                                                                        res
                                                                                                            .messages
                                                                                                            .iter()
                                                                                                            .map(
                                                                                                                |
                                                                                                                    m
                                                                                                                | m.seq
                                                                                                            )
                                                                                                            .max() else {
                                                                                                            break;
                                                                                                        };
                                                                                                    messages.extend(
                                                                                                        res
                                                                                                            .messages
                                                                                                            .into_iter()
                                                                                                            .filter(
                                                                                                                |
                                                                                                                    m
                                                                                                                | m.seq <
                                                                                                                    end
                                                                                                            )
                                                                                                    );
                                                                                                    if batch_max + 1 >=
                                                                                                        end ||
                                                                                                        (batch_len as
                                                                                                            u32) <
                                                                                                            limit {
                                                                                                        break;
                                                                                                    }
                                                                                                    next =
                                                                                                        batch_max + 1;
                                                                                                }
                                                                                                let verified =
                                                                                                    verify_pulled(
                                                                                                        state,
                                                                                                        channel,
                                                                                                        messages,
                                                                                                        None
                                                                                                    ).await?;
                                                                                                store_fetched(
                                                                                                    state,
                                                                                                    account_id,
                                                                                                    channel,
                                                                                                    local,
                                                                                                    &verified
                                                                                                ).await?;
                                                                                                return Ok(verified);
                                                                                            }.await;
                                                                                        ret
                                                                                    },)?;
                                                                                    for v in fetched {
                                                                                        merged.insert(
                                                                                            u64_to_i64(v.seq),
                                                                                            v.raw
                                                                                        );
                                                                                    }
                                                                                }
                                                                                let full = merged.len() == page_size;
                                                                                let value = flowcontrol::shed!{
                                                                                    'build _;
                                                                                    if merged.is_empty() {
                                                                                        break 'build None;
                                                                                    }
                                                                                    let mut messages =
                                                                                        Vec::with_capacity(
                                                                                            merged.len()
                                                                                        );
                                                                                    for (seq, raw) in merged {
                                                                                        let offset =
                                                                                            i64_to_u64(seq) as usize;
                                                                                        messages.push(
                                                                                            ActivityPageMessage {
                                                                                                message: serde_json
                                                                                                ::from_str::<Message>(
                                                                                                    &raw
                                                                                                ).context(
                                                                                                    "Stored message row has invalid raw JSON"
                                                                                                )?,
                                                                                                offset_pos: ActivityOffsetPos {
                                                                                                    offset: ActivityOffset(
                                                                                                        offset
                                                                                                    ),
                                                                                                    pos: page_position(
                                                                                                        offset,
                                                                                                        page_size,
                                                                                                        None
                                                                                                    ),
                                                                                                },
                                                                                            }
                                                                                        );
                                                                                    }
                                                                                    break 'build Some(
                                                                                        ActivityPageRes {
                                                                                            messages: messages
                                                                                        }
                                                                                    );
                                                                                };
                                                                                return Ok(Cached {
                                                                                    value: value,
                                                                                    policy: CachePolicy
                                                                                    ::Immutable.only_if(
                                                                                        local.public_read && full
                                                                                    ),
                                                                                });
                                                                            }.await;
                                                                        ret
                                                                    }),);
                                                                },
                                                                "calendar_page" => {
                                                                    let req =
                                                                        match c2s::GetCalendarPage::deserialize_path(
                                                                            remainder
                                                                        ) {
                                                                            Ok(r) => r,
                                                                            Err(e) => return Ok(response_400(e)),
                                                                        };
                                                                    return Ok(json_or_err(state, "calendar_page", {
                                                                        use {
                                                                            crate::subsystems::{
                                                                                calendar::{
                                                                                    calendar_local_page,
                                                                                    calendar_message,
                                                                                    stored_body,
                                                                                    CalendarLocalPage,
                                                                                },
                                                                                messages::{
                                                                                    channel_signer,
                                                                                    verify_pulled,
                                                                                    Verified,
                                                                                },
                                                                            },
                                                                            shared::interface::{
                                                                                shared::MessageRel,
                                                                                wire::c2s::{
                                                                                    CalendarPage,
                                                                                    CalendarPageRes,
                                                                                    ChannelRetention,
                                                                                },
                                                                            },
                                                                        };

                                                                        let ret:
                                                                            Result<
                                                                                Cached<Option<CalendarPageRes>>,
                                                                                GetErr
                                                                            > =
                                                                            async {
                                                                                let ctx =
                                                                                    ChannelCtx::new(
                                                                                        account,
                                                                                        req.channel.clone()
                                                                                    );
                                                                                let Some(local) = tx(&state.db, {
                                                                                    let account = ctx.account;
                                                                                    let db_owner = ctx.db_owner;
                                                                                    let db_channel_id =
                                                                                        ctx.db_channel_id.clone();
                                                                                    move |db_tx| local_channel_get(
                                                                                        db_tx,
                                                                                        account,
                                                                                        db_owner,
                                                                                        &db_channel_id
                                                                                    )
                                                                                })
                                                                                    .await
                                                                                    .context(
                                                                                        "Error reading channel for calendar"
                                                                                    )? else {
                                                                                    return Ok(Cached::no_store(None));
                                                                                };
                                                                                let local_complete =
                                                                                    local.is_owner(
                                                                                        ctx.channel.identity
                                                                                    ) ||
                                                                                        (local.retention ==
                                                                                            ChannelRetention::All &&
                                                                                            local.backfill_done);
                                                                                if local_complete {
                                                                                    let page =
                                                                                        calendar_local_page(
                                                                                            state,
                                                                                            ctx.account,
                                                                                            &ctx.channel,
                                                                                            local.page_size,
                                                                                            req.month,
                                                                                            req.page,
                                                                                            req.token
                                                                                        ).await?;
                                                                                    let value = match page {
                                                                                        CalendarLocalPage
                                                                                        ::Stale => CalendarPageRes
                                                                                        ::Stale,
                                                                                        CalendarLocalPage::Page {
                                                                                            token,
                                                                                            latest_seq,
                                                                                            roots,
                                                                                            last
                                                                                        } => {
                                                                                            let mut messages = vec![];
                                                                                            for r in roots {
                                                                                                let edit =
                                                                                                    match &r.edit {
                                                                                                        Some(e) => Some(
                                                                                                            stored_body(
                                                                                                                &e.raw
                                                                                                            )?
                                                                                                        ),
                                                                                                        None => None,
                                                                                                    };
                                                                                                messages.push(
                                                                                                    calendar_message(
                                                                                                        stored_body(
                                                                                                            &r
                                                                                                                .original
                                                                                                                .raw
                                                                                                        )?,
                                                                                                        r
                                                                                                            .original
                                                                                                            .receive_time,
                                                                                                        edit
                                                                                                    )
                                                                                                );
                                                                                            }
                                                                                            CalendarPageRes::Page(
                                                                                                CalendarPage {
                                                                                                    latest_activity: latest_seq.map(
                                                                                                        |
                                                                                                            s
                                                                                                        | activity_page_offset_pos(
                                                                                                            s,
                                                                                                            local.page_size
                                                                                                        )
                                                                                                    ),
                                                                                                    token: token,
                                                                                                    messages: messages,
                                                                                                    last: last,
                                                                                                }
                                                                                            )
                                                                                        },
                                                                                    };
                                                                                    return Ok(
                                                                                        Cached::no_store(Some(value))
                                                                                    );
                                                                                }
                                                                                if let Some(token) = req.token {
                                                                                    if let Some(cached) =
                                                                                        state
                                                                                            .calendar_page_cache
                                                                                            .get(
                                                                                                &(
                                                                                                    account,
                                                                                                    ctx
                                                                                                        .channel
                                                                                                        .clone(),
                                                                                                    req.month,
                                                                                                    req.page,
                                                                                                    token
                                                                                                )
                                                                                            )
                                                                                            .await {
                                                                                        return Ok(
                                                                                            Cached::no_store(
                                                                                                Some(
                                                                                                    CalendarPageRes
                                                                                                    ::Page(
                                                                                                        cached
                                                                                                    )
                                                                                                )
                                                                                            )
                                                                                        );
                                                                                    }
                                                                                }
                                                                                let signer =
                                                                                    channel_signer(
                                                                                        state,
                                                                                        account,
                                                                                        &local
                                                                                    ).await?;
                                                                                let Some(res) =
                                                                                    remote(
                                                                                        state
                                                                                            .s2s_client
                                                                                            .get_opt_authed(
                                                                                                ctx.channel.identity,
                                                                                                signer.as_ref(),
                                                                                                &crate
                                                                                                ::interface
                                                                                                ::s2s
                                                                                                ::GetCalendarPage {
                                                                                                    channel: ctx
                                                                                                        .channel
                                                                                                        .clone(),
                                                                                                    month: req.month,
                                                                                                    page: req.page,
                                                                                                    token: req.token,
                                                                                                }
                                                                                            )
                                                                                            .await
                                                                                            .context(
                                                                                                "Error fetching calendar page from channel owner"
                                                                                            )
                                                                                    )? else {
                                                                                        return Ok(
                                                                                            Cached::no_store(None)
                                                                                        );
                                                                                    };
                                                                                let page = match res {
                                                                                    crate
                                                                                    ::interface
                                                                                    ::s2s
                                                                                    ::CalendarPulledRes
                                                                                    ::Stale => {
                                                                                        return Ok(
                                                                                            Cached::no_store(
                                                                                                Some(
                                                                                                    CalendarPageRes
                                                                                                    ::Stale
                                                                                                )
                                                                                            )
                                                                                        );
                                                                                    },
                                                                                    crate
                                                                                    ::interface
                                                                                    ::s2s
                                                                                    ::CalendarPulledRes
                                                                                    ::Page(
                                                                                        p
                                                                                    ) => p,
                                                                                };
                                                                                let verified =
                                                                                    remote(
                                                                                        verify_pulled(
                                                                                            state,
                                                                                            &ctx.channel,
                                                                                            page.messages,
                                                                                            None
                                                                                        ).await
                                                                                    )?;
                                                                                let mut latest_edits:
                                                                                    HashMap<MessageId, &Verified> =
                                                                                    HashMap::new();
                                                                                for v in &verified {
                                                                                    let MessageRel::EditOf(target) =
                                                                                        &v.body.rel else {
                                                                                            continue;
                                                                                        };
                                                                                    let replace =
                                                                                        latest_edits
                                                                                            .get(target)
                                                                                            .map(
                                                                                                |existing| v.seq >
                                                                                                    existing.seq
                                                                                            )
                                                                                            .unwrap_or(true);
                                                                                    if replace {
                                                                                        latest_edits.insert(
                                                                                            target.clone(),
                                                                                            v
                                                                                        );
                                                                                    }
                                                                                }
                                                                                let mut messages = vec![];
                                                                                for v in &verified {
                                                                                    if let MessageRel::EditOf(_) =
                                                                                        &v.body.rel {
                                                                                        continue;
                                                                                    }
                                                                                    messages.push(
                                                                                        calendar_message(
                                                                                            v.body.clone(),
                                                                                            v.receive_time,
                                                                                            latest_edits
                                                                                                .get(&v.body.id)
                                                                                                .map(|e| e.body.clone())
                                                                                        )
                                                                                    );
                                                                                }
                                                                                let out = CalendarPage {
                                                                                    latest_activity: page
                                                                                        .latest_seq
                                                                                        .map(
                                                                                            |
                                                                                                s
                                                                                            | activity_page_offset_pos(
                                                                                                s,
                                                                                                local.page_size
                                                                                            )
                                                                                        ),
                                                                                    token: page.token,
                                                                                    messages: messages,
                                                                                    last: page.last,
                                                                                };
                                                                                state
                                                                                    .calendar_page_cache
                                                                                    .insert(
                                                                                        (
                                                                                            account,
                                                                                            ctx.channel.clone(),
                                                                                            req.month,
                                                                                            req.page,
                                                                                            out.token
                                                                                        ),
                                                                                        out.clone()
                                                                                    )
                                                                                    .await;
                                                                                return Ok(
                                                                                    Cached::no_store(
                                                                                        Some(CalendarPageRes::Page(out))
                                                                                    )
                                                                                );
                                                                            }.await;
                                                                        ret
                                                                    }));
                                                                },
                                                                "snap_page" => {
                                                                    let req =
                                                                        match GetSnapPage::deserialize_path(
                                                                            remainder
                                                                        ) {
                                                                            Ok(r) => r,
                                                                            Err(e) => return Ok(response_400(e)),
                                                                        };
                                                                    return Ok(json_or_err(state, "snap_page", {
                                                                        let ret:
                                                                            Result<
                                                                                Cached<Option<SnapPageRes>>,
                                                                                GetErr
                                                                            > =
                                                                            async {
                                                                                let (state, account, channel, page,):
                                                                                    (
                                                                                        &Arc<State>,
                                                                                        AccountId,
                                                                                        QualifiedChannelId,
                                                                                        SnapPage,
                                                                                    ) =
                                                                                    (
                                                                                        state,
                                                                                        account,
                                                                                        req.channel,
                                                                                        req.page,
                                                                                    );
                                                                                let ctx =
                                                                                    ChannelCtx::new(account, channel);
                                                                                let page = page.0 as u64;
                                                                                let Some(
                                                                                    (local, floor, originals, edits)
                                                                                ) =
                                                                                    tx(&state.db, {
                                                                                        let account = ctx.account;
                                                                                        let db_owner = ctx.db_owner;
                                                                                        let db_channel_id =
                                                                                            ctx.db_channel_id.clone();
                                                                                        move |db_tx| {
                                                                                            let Some(local) =
                                                                                                local_channel_get(
                                                                                                    db_tx,
                                                                                                    account,
                                                                                                    db_owner,
                                                                                                    &db_channel_id
                                                                                                )? else {
                                                                                                    return Ok(None);
                                                                                                };
                                                                                            let floor =
                                                                                                if local.is_owner(
                                                                                                    db_owner.0
                                                                                                ) {
                                                                                                    earliest_snap_offset(
                                                                                                        db_tx,
                                                                                                        account,
                                                                                                        db_owner,
                                                                                                        &db_channel_id
                                                                                                    )?
                                                                                                } else {
                                                                                                    local.floor_snap_offset
                                                                                                };
                                                                                            let (originals, edits) =
                                                                                                snap_page_rows(
                                                                                                    db_tx,
                                                                                                    account,
                                                                                                    db_owner,
                                                                                                    &db_channel_id,
                                                                                                    local.page_size,
                                                                                                    page
                                                                                                )?;
                                                                                            return Ok(
                                                                                                Some(
                                                                                                    (
                                                                                                        local,
                                                                                                        floor,
                                                                                                        originals,
                                                                                                        edits
                                                                                                    )
                                                                                                )
                                                                                            );
                                                                                        }
                                                                                    })
                                                                                        .await
                                                                                        .context(
                                                                                            "Error querying snap page"
                                                                                        )? else {
                                                                                        return Ok(
                                                                                            Cached::no_store(None)
                                                                                        );
                                                                                    };
                                                                                let page_size = local.page_size;
                                                                                let start =
                                                                                    u64_to_i64(
                                                                                        page * page_size as u64
                                                                                    );
                                                                                let mut originals:
                                                                                    BTreeMap<i64, MsgRow> =
                                                                                    originals
                                                                                        .into_iter()
                                                                                        .map(
                                                                                            |r| (
                                                                                                r.snap_offset.unwrap(),
                                                                                                r
                                                                                            )
                                                                                        )
                                                                                        .collect();
                                                                                let mut edits = edits;
                                                                                let latest_seq = local.latest_seq;
                                                                                let mut floor = floor;
                                                                                if !local.is_owner(
                                                                                    ctx.channel.identity
                                                                                ) &&
                                                                                    !range_complete(
                                                                                        originals.len(),
                                                                                        start,
                                                                                        page_size,
                                                                                        local.latest_snap_offset,
                                                                                        floor
                                                                                    ) {
                                                                                    let Some(fetched) =
                                                                                        remote(
                                                                                            member_fetch_snap_page(
                                                                                                state,
                                                                                                account,
                                                                                                &ctx.channel,
                                                                                                &local,
                                                                                                page
                                                                                            ).await
                                                                                        )? else {
                                                                                            return Ok(
                                                                                                Cached::no_store(None)
                                                                                            );
                                                                                        };
                                                                                    for v in &fetched {
                                                                                        let row =
                                                                                            MsgRow::from_verified(v);
                                                                                        match row.snap_offset {
                                                                                            Some(offset) => {
                                                                                                originals.insert(
                                                                                                    offset,
                                                                                                    row
                                                                                                );
                                                                                            },
                                                                                            None => {
                                                                                                edits.push(row);
                                                                                            },
                                                                                        }
                                                                                    }
                                                                                    if originals.len() < page_size {
                                                                                        let info =
                                                                                            remote(
                                                                                                member_refresh_info(
                                                                                                    state,
                                                                                                    account,
                                                                                                    &ctx.channel,
                                                                                                    &local
                                                                                                ).await
                                                                                            )?;
                                                                                        floor =
                                                                                            info
                                                                                                .earliest_snap_offset
                                                                                                .map(u64_to_i64);
                                                                                    }
                                                                                }
                                                                                let value = flowcontrol::shed!{
                                                                                    'build _;
                                                                                    if originals.is_empty() &&
                                                                                        page > 0 {
                                                                                        break 'build None;
                                                                                    }
                                                                                    let mut latest_edits: HashMap <(
                                                                                        spaghettinuum
                                                                                        ::interface
                                                                                        ::identity
                                                                                        ::Identity,
                                                                                        i64
                                                                                    ),
                                                                                    MsgRow > = HashMap:: new();
                                                                                    for e in edits {
                                                                                        let (
                                                                                            Some(target_sender),
                                                                                            Some(target_unique)
                                                                                        ) =
                                                                                            (
                                                                                                e.rel_target_sender,
                                                                                                e.rel_target_unique
                                                                                            ) else {
                                                                                                return Err(
                                                                                                    GetErr::Local(
                                                                                                        loga::err(
                                                                                                            "Edit row missing target columns"
                                                                                                        )
                                                                                                    )
                                                                                                );
                                                                                            };
                                                                                        let key =
                                                                                            (
                                                                                                target_sender,
                                                                                                target_unique
                                                                                            );
                                                                                        let replace =
                                                                                            latest_edits
                                                                                                .get(&key)
                                                                                                .map(
                                                                                                    |existing| e.seq >
                                                                                                        existing.seq
                                                                                                )
                                                                                                .unwrap_or(true);
                                                                                        if replace {
                                                                                            latest_edits.insert(
                                                                                                key,
                                                                                                e
                                                                                            );
                                                                                        }
                                                                                    }
                                                                                    let mut messages =
                                                                                        Vec::with_capacity(
                                                                                            originals.len()
                                                                                        );
                                                                                    for (_, r) in originals {
                                                                                        let offset =
                                                                                            i64_to_u64(
                                                                                                r
                                                                                                    .snap_offset
                                                                                                    .context(
                                                                                                        "Snap original without snap offset"
                                                                                                    )?
                                                                                            ) as
                                                                                                usize;
                                                                                        let origin_client_id =
                                                                                            r
                                                                                                .client_id
                                                                                                .clone()
                                                                                                .map(MessageClientId);
                                                                                        let (
                                                                                            final_body,
                                                                                            final_client_id
                                                                                        ) =
                                                                                            match latest_edits.get(
                                                                                                &(
                                                                                                    r.sender,
                                                                                                    r.sender_unique
                                                                                                )
                                                                                            ) {
                                                                                                Some(e) => (
                                                                                                    parse_blocks(
                                                                                                        &e.blocks
                                                                                                    ).map_err(
                                                                                                        GetErr::Local
                                                                                                    )?,
                                                                                                    e
                                                                                                        .client_id
                                                                                                        .clone()
                                                                                                        .map(
                                                                                                            MessageClientId
                                                                                                        )
                                                                                                        .or_else(
                                                                                                            || origin_client_id.clone()
                                                                                                        ),
                                                                                                ),
                                                                                                None => (
                                                                                                    parse_blocks(
                                                                                                        &r.blocks
                                                                                                    ).map_err(
                                                                                                        GetErr::Local
                                                                                                    )?,
                                                                                                    origin_client_id.clone()
                                                                                                ),
                                                                                            };
                                                                                        let rel =
                                                                                            match (
                                                                                                r.rel_type.as_deref(),
                                                                                                r.rel_target_sender,
                                                                                                r.rel_target_unique
                                                                                            ) {
                                                                                                (
                                                                                                    Some(REL_REPLY_TO),
                                                                                                    Some(
                                                                                                        target_sender
                                                                                                    ),
                                                                                                    Some(target_unique)
                                                                                                ) => MessageRel
                                                                                                ::ReplyTo(
                                                                                                    MessageId {
                                                                                                        identity: target_sender,
                                                                                                        unique: i64_to_u64(
                                                                                                            target_unique
                                                                                                        ),
                                                                                                    }
                                                                                                ),
                                                                                                (
                                                                                                    None,
                                                                                                    _,
                                                                                                    _
                                                                                                ) => MessageRel
                                                                                                ::None,
                                                                                                (
                                                                                                    Some(REL_EDIT_OF),
                                                                                                    _,
                                                                                                    _
                                                                                                ) => return Err(
                                                                                                    GetErr::Local(
                                                                                                        loga::err(
                                                                                                            "Edit row stored with a snap offset"
                                                                                                        )
                                                                                                    )
                                                                                                ),
                                                                                                _ => return Err(
                                                                                                    GetErr::Local(
                                                                                                        loga::err(
                                                                                                            "Snap row has inconsistent relation columns"
                                                                                                        )
                                                                                                    )
                                                                                                ),
                                                                                            };
                                                                                        let id = MessageId {
                                                                                            identity: r.sender,
                                                                                            unique: i64_to_u64(
                                                                                                r.sender_unique
                                                                                            ),
                                                                                        };
                                                                                        messages.push(SnapMessage {
                                                                                            offset_pos: SnapOffsetPos {
                                                                                                offset: SnapOffset(
                                                                                                    offset
                                                                                                ),
                                                                                                pos: page_position(
                                                                                                    offset,
                                                                                                    page_size,
                                                                                                    floor_usize(floor)
                                                                                                ),
                                                                                            },
                                                                                            original_id: QualifiedMessageId {
                                                                                                channel: ctx
                                                                                                    .channel
                                                                                                    .clone(),
                                                                                                message: id.clone(),
                                                                                            },
                                                                                            original_receive_time: r.receive_time,
                                                                                            client_id: final_client_id,
                                                                                            message: MessageBody {
                                                                                                channel: ctx
                                                                                                    .channel
                                                                                                    .clone(),
                                                                                                client_id: origin_client_id,
                                                                                                id: id,
                                                                                                rel: rel,
                                                                                                blocks: final_body,
                                                                                            },
                                                                                        });
                                                                                    }
                                                                                    break 'build Some(SnapPageRes {
                                                                                        latest_activity: latest_seq.map(
                                                                                            |s| {
                                                                                                let offset =
                                                                                                    i64_to_u64(s) as
                                                                                                        usize;
                                                                                                return ActivityPageOffsetPos {
                                                                                                    page: ActivityPage(
                                                                                                        offset /
                                                                                                            page_size
                                                                                                    ),
                                                                                                    offset_pos: ActivityOffsetPos {
                                                                                                        offset: ActivityOffset(
                                                                                                            offset
                                                                                                        ),
                                                                                                        pos: page_position(
                                                                                                            offset,
                                                                                                            page_size,
                                                                                                            None
                                                                                                        ),
                                                                                                    },
                                                                                                };
                                                                                            }
                                                                                        ),
                                                                                        messages: messages,
                                                                                    });
                                                                                };
                                                                                return Ok(Cached {
                                                                                    value: value,
                                                                                    policy: CachePolicy::MaxAge(
                                                                                        SNAP_PAGE_MAX_AGE_SECS
                                                                                    ).only_if(local.public_read),
                                                                                });
                                                                            }.await;
                                                                        ret
                                                                    }));
                                                                },
                                                                "snap_by_id" => {
                                                                    let req =
                                                                        match SnapById::deserialize_path(remainder) {
                                                                            Ok(r) => r,
                                                                            Err(e) => return Ok(response_400(e)),
                                                                        };
                                                                    return Ok(json_or_err(state, "snap_by_id", {
                                                                        let ret:
                                                                            Result<Cached<Option<SnapByRes>>, GetErr> =
                                                                            async {
                                                                                let (state, account, id,):
                                                                                    (
                                                                                        &Arc<State>,
                                                                                        AccountId,
                                                                                        QualifiedMessageId,
                                                                                    ) =
                                                                                    (state, account, req.id,);
                                                                                let ctx =
                                                                                    ChannelCtx::new(
                                                                                        account,
                                                                                        id.channel.clone()
                                                                                    );
                                                                                let message = id.message;
                                                                                let Some((local, floor, pos)) =
                                                                                    tx(&state.db, {
                                                                                        let account = ctx.account;
                                                                                        let db_owner = ctx.db_owner;
                                                                                        let db_channel_id =
                                                                                            ctx.db_channel_id.clone();
                                                                                        let message = message.clone();
                                                                                        move |db_tx| {
                                                                                            let Some(local) =
                                                                                                local_channel_get(
                                                                                                    db_tx,
                                                                                                    account,
                                                                                                    db_owner,
                                                                                                    &db_channel_id
                                                                                                )? else {
                                                                                                    return Ok(None);
                                                                                                };
                                                                                            let floor =
                                                                                                channel_floor(
                                                                                                    db_tx,
                                                                                                    account,
                                                                                                    db_owner,
                                                                                                    &db_channel_id,
                                                                                                    &local
                                                                                                )?;
                                                                                            let pos =
                                                                                                snap_position_by_id(
                                                                                                    db_tx,
                                                                                                    account,
                                                                                                    db_owner,
                                                                                                    &db_channel_id,
                                                                                                    &message
                                                                                                )?;
                                                                                            return Ok(
                                                                                                Some(
                                                                                                    (local, floor, pos)
                                                                                                )
                                                                                            );
                                                                                        }
                                                                                    })
                                                                                        .await
                                                                                        .context(
                                                                                            "Error looking up snap position"
                                                                                        )? else {
                                                                                        return Ok(
                                                                                            Cached::no_store(None)
                                                                                        );
                                                                                    };
                                                                                if let Some(pos) = pos {
                                                                                    return Ok(Cached {
                                                                                        value: Some(
                                                                                            snap_by_res(
                                                                                                pos,
                                                                                                local.page_size,
                                                                                                floor
                                                                                            )
                                                                                        ),
                                                                                        policy: CachePolicy
                                                                                        ::Immutable.only_if(
                                                                                            local.public_read
                                                                                        ),
                                                                                    });
                                                                                }
                                                                                if local.is_owner(
                                                                                    ctx.channel.identity
                                                                                ) {
                                                                                    return Ok(Cached::no_store(None));
                                                                                }
                                                                                let Some(found) =
                                                                                    remote(
                                                                                        member_fetch_snap_lookup(
                                                                                            state,
                                                                                            account,
                                                                                            &ctx.channel,
                                                                                            &local,
                                                                                            SnapLookupBy::Id(message)
                                                                                        ).await,
                                                                                    )? else {
                                                                                        return Ok(
                                                                                            Cached::no_store(None)
                                                                                        );
                                                                                    };
                                                                                return Ok(Cached {
                                                                                    value: Some(
                                                                                        snap_by_res(SnapPosition {
                                                                                            original_id: found.original_id,
                                                                                            snap_offset: u64_to_i64(
                                                                                                found.snap_offset
                                                                                            ),
                                                                                            receive_time: found.receive_time,
                                                                                        }, local.page_size, floor)
                                                                                    ),
                                                                                    policy: CachePolicy
                                                                                    ::Immutable.only_if(
                                                                                        local.public_read
                                                                                    ),
                                                                                });
                                                                            }.await;
                                                                        ret
                                                                    }));
                                                                },
                                                                "snap_by_client_id" => {
                                                                    let req =
                                                                        match SnapByClientId::deserialize_path(
                                                                            remainder
                                                                        ) {
                                                                            Ok(r) => r,
                                                                            Err(e) => return Ok(response_400(e)),
                                                                        };
                                                                    return Ok(json_or_err(state, "snap_by_client_id", {
                                                                        let ret:
                                                                            Result<Cached<Option<SnapByRes>>, GetErr> =
                                                                            async {
                                                                                let (
                                                                                    state,
                                                                                    account,
                                                                                    channel,
                                                                                    client_id,
                                                                                ):
                                                                                    (
                                                                                        &Arc<State>,
                                                                                        AccountId,
                                                                                        QualifiedChannelId,
                                                                                        MessageClientId,
                                                                                    ) =
                                                                                    (
                                                                                        state,
                                                                                        account,
                                                                                        req.channel,
                                                                                        req.client_id,
                                                                                    );
                                                                                let ctx =
                                                                                    ChannelCtx::new(account, channel);
                                                                                let Some((local, floor, pos)) =
                                                                                    tx(&state.db, {
                                                                                        let account = ctx.account;
                                                                                        let db_owner = ctx.db_owner;
                                                                                        let db_channel_id =
                                                                                            ctx.db_channel_id.clone();
                                                                                        let client_id =
                                                                                            client_id.0.clone();
                                                                                        move |db_tx| {
                                                                                            let Some(local) =
                                                                                                local_channel_get(
                                                                                                    db_tx,
                                                                                                    account,
                                                                                                    db_owner,
                                                                                                    &db_channel_id
                                                                                                )? else {
                                                                                                    return Ok(None);
                                                                                                };
                                                                                            let floor =
                                                                                                channel_floor(
                                                                                                    db_tx,
                                                                                                    account,
                                                                                                    db_owner,
                                                                                                    &db_channel_id,
                                                                                                    &local
                                                                                                )?;
                                                                                            let pos =
                                                                                                snap_position_by_client_id(
                                                                                                    db_tx,
                                                                                                    account,
                                                                                                    db_owner,
                                                                                                    &db_channel_id,
                                                                                                    local.local_identity,
                                                                                                    &client_id,
                                                                                                )?;
                                                                                            return Ok(
                                                                                                Some(
                                                                                                    (local, floor, pos)
                                                                                                )
                                                                                            );
                                                                                        }
                                                                                    })
                                                                                        .await
                                                                                        .context(
                                                                                            "Error looking up message by client_id"
                                                                                        )? else {
                                                                                        return Ok(
                                                                                            Cached::no_store(None)
                                                                                        );
                                                                                    };
                                                                                if let Some(pos) = pos {
                                                                                    return Ok(Cached {
                                                                                        value: Some(
                                                                                            snap_by_res(
                                                                                                pos,
                                                                                                local.page_size,
                                                                                                floor
                                                                                            )
                                                                                        ),
                                                                                        policy: CachePolicy
                                                                                        ::Immutable.only_if(
                                                                                            local.public_read
                                                                                        ),
                                                                                    });
                                                                                }
                                                                                if local.is_owner(
                                                                                    ctx.channel.identity
                                                                                ) {
                                                                                    return Ok(Cached::no_store(None));
                                                                                }
                                                                                let Some(found) =
                                                                                    remote(
                                                                                        member_fetch_snap_lookup(
                                                                                            state,
                                                                                            account,
                                                                                            &ctx.channel,
                                                                                            &local,
                                                                                            SnapLookupBy::ClientId(
                                                                                                local.local_identity,
                                                                                                client_id
                                                                                            ),
                                                                                        ).await,
                                                                                    )? else {
                                                                                        return Ok(
                                                                                            Cached::no_store(None)
                                                                                        );
                                                                                    };
                                                                                return Ok(Cached {
                                                                                    value: Some(
                                                                                        snap_by_res(SnapPosition {
                                                                                            original_id: found.original_id,
                                                                                            snap_offset: u64_to_i64(
                                                                                                found.snap_offset
                                                                                            ),
                                                                                            receive_time: found.receive_time,
                                                                                        }, local.page_size, floor)
                                                                                    ),
                                                                                    policy: CachePolicy
                                                                                    ::Immutable.only_if(
                                                                                        local.public_read
                                                                                    ),
                                                                                });
                                                                            }.await;
                                                                        ret
                                                                    },),);
                                                                },
                                                                "snap_page_containing_time" => {
                                                                    let req =
                                                                        match SnapPageContainingTime::deserialize_path(
                                                                            remainder
                                                                        ) {
                                                                            Ok(r) => r,
                                                                            Err(e) => return Ok(response_400(e)),
                                                                        };
                                                                    return Ok(
                                                                        json_or_err(
                                                                            state,
                                                                            "snap_page_containing_time",
                                                                            no_store({
                                                                                let ret:
                                                                                    Result<Option<SnapPage>, GetErr> =
                                                                                    async {
                                                                                        let (
                                                                                            state,
                                                                                            account,
                                                                                            channel,
                                                                                            time,
                                                                                        ):
                                                                                            (
                                                                                                &Arc<State>,
                                                                                                AccountId,
                                                                                                QualifiedChannelId,
                                                                                                jiff::Timestamp,
                                                                                            ) =
                                                                                            (
                                                                                                state,
                                                                                                account,
                                                                                                req.channel,
                                                                                                req.time,
                                                                                            );
                                                                                        let ctx =
                                                                                            ChannelCtx::new(
                                                                                                account,
                                                                                                channel
                                                                                            );
                                                                                        let Some(local) =
                                                                                            tx(&state.db, {
                                                                                                let account =
                                                                                                    ctx.account;
                                                                                                let db_owner =
                                                                                                    ctx.db_owner;
                                                                                                let db_channel_id =
                                                                                                    ctx
                                                                                                        .db_channel_id
                                                                                                        .clone();
                                                                                                move |db_tx| {
                                                                                                    return local_channel_get(
                                                                                                        db_tx,
                                                                                                        account,
                                                                                                        db_owner,
                                                                                                        &db_channel_id
                                                                                                    );
                                                                                                }
                                                                                            })
                                                                                                .await
                                                                                                .context(
                                                                                                    "Error loading channel"
                                                                                                )? else {
                                                                                                return Ok(None);
                                                                                            };
                                                                                        let page_size =
                                                                                            local.page_size;
                                                                                        if local.is_owner(
                                                                                            ctx.channel.identity
                                                                                        ) ||
                                                                                            local.backfill_done {
                                                                                            let offset =
                                                                                                tx(&state.db, {
                                                                                                    let account =
                                                                                                        ctx.account;
                                                                                                    let db_owner =
                                                                                                        ctx.db_owner;
                                                                                                    let db_channel_id =
                                                                                                        ctx
                                                                                                            .db_channel_id
                                                                                                            .clone();
                                                                                                    move |db_tx| {
                                                                                                        return snap_offset_containing_time(
                                                                                                            db_tx,
                                                                                                            account,
                                                                                                            db_owner,
                                                                                                            &db_channel_id,
                                                                                                            time
                                                                                                        );
                                                                                                    }
                                                                                                })
                                                                                                    .await
                                                                                                    .context(
                                                                                                        "Error finding snap page for time"
                                                                                                    )?;
                                                                                            if local.is_owner(
                                                                                                ctx.channel.identity
                                                                                            ) {
                                                                                                return Ok(
                                                                                                    offset.map(
                                                                                                        |o| SnapPage(
                                                                                                            i64_to_u64(
                                                                                                                o
                                                                                                            ) as
                                                                                                                usize /
                                                                                                                page_size
                                                                                                        )
                                                                                                    )
                                                                                                );
                                                                                            }
                                                                                            if let Some(o) = offset {
                                                                                                return Ok(
                                                                                                    Some(
                                                                                                        SnapPage(
                                                                                                            i64_to_u64(
                                                                                                                o
                                                                                                            ) as
                                                                                                                usize /
                                                                                                                page_size
                                                                                                        )
                                                                                                    )
                                                                                                );
                                                                                            }
                                                                                        }
                                                                                        return Ok(
                                                                                            remote(
                                                                                                {
                                                                                                    use {
                                                                                                        crate::State,
                                                                                                        jiff::Timestamp,
                                                                                                        loga::ResultContext,
                                                                                                        shared::interface::shared::{
                                                                                                            AccountId,
                                                                                                            QualifiedChannelId,
                                                                                                        },
                                                                                                        std::sync::Arc,
                                                                                                    };
                                                                                                    use crate::subsystems::messages::*;

                                                                                                    let ret:
                                                                                                        Result<
                                                                                                            Option<u64>,
                                                                                                            loga::Error
                                                                                                        > =
                                                                                                        async {
                                                                                                            let (
                                                                                                                state,
                                                                                                                account_id,
                                                                                                                channel,
                                                                                                                local,
                                                                                                                time,
                                                                                                            ):
                                                                                                                (
                                                                                                                    &Arc<
                                                                                                                        State
                                                                                                                    >,
                                                                                                                    AccountId,
                                                                                                                    &QualifiedChannelId,
                                                                                                                    &LocalChannel,
                                                                                                                    Timestamp,
                                                                                                                ) =
                                                                                                                (
                                                                                                                    state,
                                                                                                                    account,
                                                                                                                    &ctx.channel,
                                                                                                                    &local,
                                                                                                                    time,
                                                                                                                );
                                                                                                            let signer =
                                                                                                                channel_signer(
                                                                                                                    state,
                                                                                                                    account_id,
                                                                                                                    local
                                                                                                                ).await?;
                                                                                                            return {
                                                                                                                use {
                                                                                                                    crate::interface::s2s::GetSnapPageContainingTime,
                                                                                                                    spaghettinuum::interface::identity::Identity,
                                                                                                                };
                                                                                                                use crate::subsystems::s2s_client::*;

                                                                                                                let ret:
                                                                                                                    Result<
                                                                                                                        Option<
                                                                                                                            u64
                                                                                                                        >,
                                                                                                                        loga
                                                                                                                        ::Error
                                                                                                                    > =
                                                                                                                    async {
                                                                                                                        let (
                                                                                                                            s2s_client,
                                                                                                                            target,
                                                                                                                            signer,
                                                                                                                            req,
                                                                                                                        ):
                                                                                                                            (
                                                                                                                                &S2sClient,
                                                                                                                                Identity,
                                                                                                                                Option<
                                                                                                                                    &Signer
                                                                                                                                >,
                                                                                                                                GetSnapPageContainingTime,
                                                                                                                            ) =
                                                                                                                            (
                                                                                                                                &(state.s2s_client),
                                                                                                                                channel.identity,
                                                                                                                                signer.as_ref(),
                                                                                                                                GetSnapPageContainingTime {
                                                                                                                                    channel: channel.clone(),
                                                                                                                                    time: time,
                                                                                                                                },
                                                                                                                            );
                                                                                                                        return s2s_client
                                                                                                                            .get_opt_authed(
                                                                                                                                target,
                                                                                                                                signer,
                                                                                                                                &req
                                                                                                                            )
                                                                                                                            .await;
                                                                                                                    }.await;
                                                                                                                ret
                                                                                                            }.context(
                                                                                                                "Error looking up snap page by time from channel owner"
                                                                                                            );
                                                                                                        }.await;
                                                                                                    ret
                                                                                                },
                                                                                            )?.map(
                                                                                                |p| SnapPage(p as usize)
                                                                                            ),
                                                                                        );
                                                                                    }.await;
                                                                                ret
                                                                            }),
                                                                        ),
                                                                    );
                                                                },
                                                                "outbox" => {
                                                                    let req =
                                                                        match OutboxList::deserialize_path(
                                                                            remainder
                                                                        ) {
                                                                            Ok(r) => r,
                                                                            Err(e) => return Ok(response_400(e)),
                                                                        };
                                                                    return Ok(json_or_err(state, "outbox", no_store({
                                                                        let ret:
                                                                            Result<Vec<OutboxEntry>, loga::Error> =
                                                                            async {
                                                                                let (state, account, channel,):
                                                                                    (
                                                                                        &Arc<State>,
                                                                                        AccountId,
                                                                                        QualifiedChannelId,
                                                                                    ) =
                                                                                    (state, account, req.channel,);
                                                                                let ctx =
                                                                                    ChannelCtx::new(account, channel);
                                                                                let rows =
                                                                                    tx(&state.db, {
                                                                                        let account = ctx.account;
                                                                                        let db_owner = ctx.db_owner;
                                                                                        let db_channel_id =
                                                                                            ctx.db_channel_id.clone();
                                                                                        move |db_tx| {
                                                                                            return Ok(
                                                                                                good_ormning
                                                                                                ::sqlite
                                                                                                ::good_query_many!(
                                                                                                    db,
                                                                                                    r#"select sender, client_id, created, raw from outbox where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} order by sender_unique"#;
                                                                                                    db_tx
                                                                                                ).map_err(
                                                                                                    |e| loga::err(e.0)
                                                                                                )?
                                                                                            );
                                                                                        }
                                                                                    })
                                                                                        .await
                                                                                        .context(
                                                                                            "Error listing outbox"
                                                                                        )?;
                                                                                let mut out =
                                                                                    Vec::with_capacity(rows.len());
                                                                                for r in rows {
                                                                                    let message =
                                                                                        serde_json
                                                                                        ::from_str::<Message>(
                                                                                            &r.raw
                                                                                        ).context(
                                                                                            "Stored outbox message was not valid JSON"
                                                                                        )?;
                                                                                    let body =
                                                                                        serde_json
                                                                                        ::from_str::<MessageBody>(
                                                                                            &message.0.message,
                                                                                        ).context(
                                                                                            "Stored outbox body was not valid JSON"
                                                                                        )?;
                                                                                    out.push(OutboxEntry {
                                                                                        sender: r.sender.0,
                                                                                        client_id: MessageClientId(
                                                                                            r.client_id
                                                                                        ),
                                                                                        created: r.created.0,
                                                                                        id: body.id,
                                                                                        rel: body.rel,
                                                                                        blocks: body.blocks,
                                                                                    });
                                                                                }
                                                                                return Ok(out);
                                                                            }.await;
                                                                        ret
                                                                    }.map_err(GetErr::Local)),),);
                                                                },
                                                                "message_history" => {
                                                                    let req =
                                                                        match MessageHistory::deserialize_path(
                                                                            remainder
                                                                        ) {
                                                                            Ok(r) => r,
                                                                            Err(e) => return Ok(response_400(e)),
                                                                        };
                                                                    return Ok(
                                                                        json_or_err(state, "message_history", no_store({
                                                                            let ret:
                                                                                Result<
                                                                                    Option<Vec<MessageRevision>>,
                                                                                    GetErr
                                                                                > =
                                                                                async {
                                                                                    let (state, account, id,):
                                                                                        (
                                                                                            &Arc<State>,
                                                                                            AccountId,
                                                                                            QualifiedMessageId,
                                                                                        ) =
                                                                                        (state, account, req.id,);
                                                                                    let ctx =
                                                                                        ChannelCtx::new(
                                                                                            account,
                                                                                            id.channel.clone()
                                                                                        );
                                                                                    let message = id.message;
                                                                                    let Some(
                                                                                        (local, rows, outbox_edits)
                                                                                    ) =
                                                                                        tx(&state.db, {
                                                                                            let account = ctx.account;
                                                                                            let db_owner =
                                                                                                ctx.db_owner;
                                                                                            let db_channel_id =
                                                                                                ctx
                                                                                                    .db_channel_id
                                                                                                    .clone();
                                                                                            let message =
                                                                                                message.clone();
                                                                                            move |db_tx| {
                                                                                                let Some(local) =
                                                                                                    local_channel_get(
                                                                                                        db_tx,
                                                                                                        account,
                                                                                                        db_owner,
                                                                                                        &db_channel_id
                                                                                                    )? else {
                                                                                                        return Ok(
                                                                                                            None
                                                                                                        );
                                                                                                    };
                                                                                                if !local.is_owner(
                                                                                                    db_owner.0
                                                                                                ) {
                                                                                                    let rows =
                                                                                                        good_ormning
                                                                                                        ::sqlite
                                                                                                        ::good_query_many!(
                                                                                                            db,
                                                                                                            r#"select client_id, created, raw from outbox where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} order by sender_unique"#;
                                                                                                            db_tx
                                                                                                        ).map_err(
                                                                                                            |e| loga
                                                                                                            ::err(
                                                                                                                e.0
                                                                                                            )
                                                                                                        )?;
                                                                                                    let mut outbox_edits =
                                                                                                        vec![];
                                                                                                    for r in rows {
                                                                                                        let raw_message =
                                                                                                            serde_json
                                                                                                            ::from_str::<
                                                                                                                Message
                                                                                                            >(
                                                                                                                &r.raw,
                                                                                                            ).context(
                                                                                                                "Stored outbox message was not valid JSON"
                                                                                                            )?;
                                                                                                        let body =
                                                                                                            serde_json
                                                                                                            ::from_str::<
                                                                                                                MessageBody
                                                                                                            >(
                                                                                                                &raw_message
                                                                                                                    .0
                                                                                                                    .message,
                                                                                                            ).context(
                                                                                                                "Stored outbox body was not valid JSON"
                                                                                                            )?;
                                                                                                        let MessageRel
                                                                                                        ::EditOf(
                                                                                                            edit_target
                                                                                                        ) =
                                                                                                            body.rel else {
                                                                                                                continue;
                                                                                                            };
                                                                                                        if edit_target !=
                                                                                                            message {
                                                                                                            continue;
                                                                                                        }
                                                                                                        outbox_edits.push(
                                                                                                            OutboxEdit {
                                                                                                                created: r
                                                                                                                    .created
                                                                                                                    .0,
                                                                                                                client_id: r.client_id,
                                                                                                                blocks: body.blocks,
                                                                                                            }
                                                                                                        );
                                                                                                    }
                                                                                                    return Ok(
                                                                                                        Some(
                                                                                                            (
                                                                                                                local,
                                                                                                                None,
                                                                                                                outbox_edits
                                                                                                            )
                                                                                                        )
                                                                                                    );
                                                                                                }
                                                                                                let rows =
                                                                                                    message_history_rows(
                                                                                                        db_tx,
                                                                                                        account,
                                                                                                        db_owner,
                                                                                                        &db_channel_id,
                                                                                                        &message
                                                                                                    )?;
                                                                                                return Ok(
                                                                                                    Some(
                                                                                                        (
                                                                                                            local,
                                                                                                            rows,
                                                                                                            vec![]
                                                                                                        )
                                                                                                    )
                                                                                                );
                                                                                            }
                                                                                        })
                                                                                            .await
                                                                                            .context(
                                                                                                "Error querying message history"
                                                                                            )? else {
                                                                                            return Ok(None);
                                                                                        };
                                                                                    if local.is_owner(
                                                                                        ctx.channel.identity
                                                                                    ) {
                                                                                        let Some(rows) = rows else {
                                                                                            return Ok(None);
                                                                                        };
                                                                                        return Ok(
                                                                                            build_message_history(
                                                                                                &message,
                                                                                                rows,
                                                                                                outbox_edits
                                                                                            )?
                                                                                        );
                                                                                    }
                                                                                    let Some(fetched) = remote({
                                                                                        use {
                                                                                            crate::State,
                                                                                            loga::ResultContext,
                                                                                            shared::interface::shared::{
                                                                                                AccountId,
                                                                                                MessageId,
                                                                                                QualifiedChannelId,
                                                                                            },
                                                                                            std::sync::Arc,
                                                                                        };
                                                                                        use crate::subsystems::messages::*;

                                                                                        let ret:
                                                                                            Result<
                                                                                                Option<Vec<Verified>>,
                                                                                                loga::Error
                                                                                            > =
                                                                                            async {
                                                                                                let (
                                                                                                    state,
                                                                                                    account_id,
                                                                                                    channel,
                                                                                                    local,
                                                                                                    id,
                                                                                                ):
                                                                                                    (
                                                                                                        &Arc<State>,
                                                                                                        AccountId,
                                                                                                        &QualifiedChannelId,
                                                                                                        &LocalChannel,
                                                                                                        &MessageId,
                                                                                                    ) =
                                                                                                    (
                                                                                                        state,
                                                                                                        account,
                                                                                                        &ctx.channel,
                                                                                                        &local,
                                                                                                        &message,
                                                                                                    );
                                                                                                let signer =
                                                                                                    channel_signer(
                                                                                                        state,
                                                                                                        account_id,
                                                                                                        local
                                                                                                    ).await?;
                                                                                                let Some(res) = {
                                                                                                    use {
                                                                                                        crate::interface::s2s::{
                                                                                                            GetMessageHistory,
                                                                                                            PulledMessages,
                                                                                                        },
                                                                                                        spaghettinuum::interface::identity::Identity,
                                                                                                    };
                                                                                                    use crate::subsystems::s2s_client::*;

                                                                                                    let ret:
                                                                                                        Result<
                                                                                                            Option<
                                                                                                                PulledMessages
                                                                                                            >,
                                                                                                            loga::Error
                                                                                                        > =
                                                                                                        async {
                                                                                                            let (
                                                                                                                s2s_client,
                                                                                                                target,
                                                                                                                signer,
                                                                                                                req,
                                                                                                            ):
                                                                                                                (
                                                                                                                    &S2sClient,
                                                                                                                    Identity,
                                                                                                                    Option<
                                                                                                                        &Signer
                                                                                                                    >,
                                                                                                                    GetMessageHistory,
                                                                                                                ) =
                                                                                                                (
                                                                                                                    &(state.s2s_client),
                                                                                                                    channel.identity,
                                                                                                                    signer.as_ref(),
                                                                                                                    GetMessageHistory {
                                                                                                                        channel: channel.clone(),
                                                                                                                        id: id.clone(),
                                                                                                                    },
                                                                                                                );
                                                                                                            return s2s_client
                                                                                                                .get_opt_authed(
                                                                                                                    target,
                                                                                                                    signer,
                                                                                                                    &req
                                                                                                                )
                                                                                                                .await;
                                                                                                        }.await;
                                                                                                    ret
                                                                                                }.context(
                                                                                                    "Error fetching message history from channel owner"
                                                                                                )? else {
                                                                                                    return Ok(None);
                                                                                                };
                                                                                                let verified =
                                                                                                    verify_pulled(
                                                                                                        state,
                                                                                                        channel,
                                                                                                        res.messages,
                                                                                                        None
                                                                                                    ).await?;
                                                                                                store_fetched(
                                                                                                    state,
                                                                                                    account_id,
                                                                                                    channel,
                                                                                                    local,
                                                                                                    &verified
                                                                                                ).await?;
                                                                                                return Ok(
                                                                                                    Some(verified)
                                                                                                );
                                                                                            }.await;
                                                                                        ret
                                                                                    })? else {
                                                                                        return Ok(None);
                                                                                    };
                                                                                    return Ok(
                                                                                        build_message_history(
                                                                                            &message,
                                                                                            fetched
                                                                                                .iter()
                                                                                                .map(
                                                                                                    MsgRow
                                                                                                    ::from_verified
                                                                                                )
                                                                                                .collect(),
                                                                                            outbox_edits
                                                                                        )?
                                                                                    );
                                                                                }.await;
                                                                            ret
                                                                        })),
                                                                    );
                                                                },
                                                                "message_id_by_client_id" => {
                                                                    let req =
                                                                        match MessageIdByClientId::deserialize_path(
                                                                            remainder
                                                                        ) {
                                                                            Ok(r) => r,
                                                                            Err(e) => return Ok(response_400(e)),
                                                                        };
                                                                    return Ok(
                                                                        json_or_err(
                                                                            state,
                                                                            "message_id_by_client_id",
                                                                            no_store({
                                                                                let ret:
                                                                                    Result<Option<MessageId>, GetErr> =
                                                                                    async {
                                                                                        let (
                                                                                            state,
                                                                                            account,
                                                                                            channel,
                                                                                            client_id,
                                                                                        ):
                                                                                            (
                                                                                                &Arc<State>,
                                                                                                AccountId,
                                                                                                QualifiedChannelId,
                                                                                                MessageClientId,
                                                                                            ) =
                                                                                            (
                                                                                                state,
                                                                                                account,
                                                                                                req.channel,
                                                                                                req.client_id,
                                                                                            );
                                                                                        let ctx =
                                                                                            ChannelCtx::new(
                                                                                                account,
                                                                                                channel
                                                                                            );
                                                                                        let Some((local, found)) =
                                                                                            tx(&state.db, {
                                                                                                let account =
                                                                                                    ctx.account;
                                                                                                let db_owner =
                                                                                                    ctx.db_owner;
                                                                                                let db_channel_id =
                                                                                                    ctx
                                                                                                        .db_channel_id
                                                                                                        .clone();
                                                                                                let client_id =
                                                                                                    client_id
                                                                                                        .0
                                                                                                        .clone();
                                                                                                move |db_tx| {
                                                                                                    let Some(local) =
                                                                                                        local_channel_get(
                                                                                                            db_tx,
                                                                                                            account,
                                                                                                            db_owner,
                                                                                                            &db_channel_id
                                                                                                        )? else {
                                                                                                            return Ok(
                                                                                                                None
                                                                                                            );
                                                                                                        };
                                                                                                    let db_sender =
                                                                                                        DbIdentity(
                                                                                                            local.local_identity
                                                                                                        );
                                                                                                    let outbox =
                                                                                                        good_ormning
                                                                                                        ::sqlite
                                                                                                        ::good_query_opt!(
                                                                                                            db,
                                                                                                            r#"select sender_unique from outbox where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and sender = ${identity_id_t = &db_sender} and client_id = ${string = client_id.as_str()}"#;
                                                                                                            db_tx
                                                                                                        ).map_err(
                                                                                                            |e| loga
                                                                                                            ::err(
                                                                                                                e.0
                                                                                                            )
                                                                                                        )?;
                                                                                                    if let Some(
                                                                                                        unique
                                                                                                    ) =
                                                                                                        outbox {
                                                                                                        return Ok(
                                                                                                            Some(
                                                                                                                (
                                                                                                                    local.clone(),
                                                                                                                    Some(
                                                                                                                        MessageId {
                                                                                                                            identity: local.local_identity,
                                                                                                                            unique: i64_to_u64(
                                                                                                                                unique
                                                                                                                            ),
                                                                                                                        }
                                                                                                                    )
                                                                                                                )
                                                                                                            )
                                                                                                        );
                                                                                                    }
                                                                                                    let stored =
                                                                                                        good_ormning
                                                                                                        ::sqlite
                                                                                                        ::good_query_opt!(
                                                                                                            db,
                                                                                                            r#"select sender_unique from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and sender = ${identity_id_t = &db_sender} and client_id = ${string = client_id.as_str()}"#;
                                                                                                            db_tx
                                                                                                        ).map_err(
                                                                                                            |e| loga
                                                                                                            ::err(
                                                                                                                e.0
                                                                                                            )
                                                                                                        )?;
                                                                                                    return Ok(
                                                                                                        Some(
                                                                                                            (
                                                                                                                local.clone(),
                                                                                                                stored.map(
                                                                                                                    |
                                                                                                                        unique
                                                                                                                    | MessageId {
                                                                                                                        identity: local.local_identity,
                                                                                                                        unique: i64_to_u64(
                                                                                                                            unique
                                                                                                                        ),
                                                                                                                    }
                                                                                                                )
                                                                                                            )
                                                                                                        )
                                                                                                    );
                                                                                                }
                                                                                            })
                                                                                                .await
                                                                                                .context(
                                                                                                    "Error looking up message id by client_id"
                                                                                                )? else {
                                                                                                return Ok(None);
                                                                                            };
                                                                                        if let Some(found) = found {
                                                                                            return Ok(Some(found));
                                                                                        }
                                                                                        if local.is_owner(
                                                                                            ctx.channel.identity
                                                                                        ) {
                                                                                            return Ok(None);
                                                                                        }
                                                                                        let found =
                                                                                            remote(
                                                                                                member_fetch_snap_lookup(
                                                                                                    state,
                                                                                                    account,
                                                                                                    &ctx.channel,
                                                                                                    &local,
                                                                                                    SnapLookupBy
                                                                                                    ::ClientId(
                                                                                                        local.local_identity,
                                                                                                        client_id
                                                                                                    ),
                                                                                                ).await,
                                                                                            )?;
                                                                                        return Ok(
                                                                                            found.map(|f| f.original_id)
                                                                                        );
                                                                                    }.await;
                                                                                ret
                                                                            }),
                                                                        ),
                                                                    );
                                                                },
                                                                _ => return Ok(response_404()),
                                                            }
                                                        }.await;
                                                        ret
                                                    };
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
