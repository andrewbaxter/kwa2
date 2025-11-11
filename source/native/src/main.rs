pub mod interface;
pub mod dbutil;
pub mod fsutil;
pub mod db;
pub mod subsystems;
pub mod util;

use {
    crate::{
        dbutil::tx,
        fsutil::create_dirs,
        interface::{
            config::OidcConfig,
            s2s::s2sv1t::GetLastPageRes,
            AccountExternalId,
        },
        subsystems::oidc::{
            self,
            get_req_session,
            OidcState,
        },
    },
    aargvark::{
        traits_impls::AargvarkJson,
        vark,
        Aargvark,
    },
    cookie::Cookie,
    deadpool_sqlite::Pool,
    flowcontrol::{
        shed,
        ta_return,
    },
    glove::reqresp,
    http::{
        header::{
            COOKIE,
            ETAG,
            IF_NONE_MATCH,
        },
        status,
        HeaderMap,
        Method,
        Request,
        Response,
    },
    http_body_util::{
        combinators::BoxBody,
        BodyExt,
    },
    htwrap::{
        handler,
        htserve::{
            self,
            handler::{
                Handler,
                PathRouter,
            },
            responses::{
                body_full,
                response_200_json,
                response_400,
                response_401,
                response_403,
                response_404,
                response_503,
                Body,
            },
            viserr::{
                ResultVisErr,
                VisErr,
            },
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
    rand::rng,
    rust_embed::RustEmbed,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::wire::{
        c2s::{
            self,
            ChannelGroupRes,
            ChannelOrChannelGroup,
            ChannelOrChannelGroupGroup,
            ChannelRes,
        },
    },
    spaghettinuum::interface::identity::Identity,
    spaghettinuum_native::{
        interface::config::shared::{
            GlobalAddrConfig,
            StrSocketAddr,
        },
        utils::system_addr::resolve_global_ip,
    },
    std::{
        collections::BTreeMap,
        net::SocketAddr,
        path::PathBuf,
        str::FromStr,
        sync::Arc,
        time::Duration,
    },
    taskmanager::TaskManager,
    tokio::{
        net::{
            TcpListener,
            TcpStream,
        },
        runtime,
        select,
        spawn,
        sync::mpsc,
    },
    tokio_stream::wrappers::TcpListenerStream,
};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Config {
    pub bind_sockaddr: StrSocketAddr,
    pub spagh_node_bind_sockaddr: StrSocketAddr,
    pub spagh_publisher_bind_sockaddr: StrSocketAddr,
    pub spagh_publisher_advertise_global_addr: GlobalAddrConfig,
    pub spagh_publisher_advertise_global_port: Option<u16>,
    pub public_http_resp_cache_duration: Duration,
    pub cache_dir: PathBuf,
    pub persistent_dir: PathBuf,
    pub oidc_config: OidcConfig,
}

#[derive(Aargvark)]
struct Args {
    config: AargvarkJson<Config>,
}

struct State {
    log: loga::Log,
    db: Pool,
    oidc_state: OidcState,
}

pub async fn identify_c2s(
    state: &State,
    headers: &HeaderMap,
) -> Result<Option<AccountExternalId>, VisErr<loga::Error>> {
    shed!{
        let Some(session) = get_req_session(&state.log, headers) else {
            break;
        };
        let Some(user) = state.oidc_state.sessions.get(&session).await else {
            state
                .log
                .log(loga::DEBUG, format!("Request has session id [{}] but no matching session found", session));
            break;
        };
        return Ok(Some(user));
    }
    return Ok(None);
}

async fn handle_req(state: &Arc<State>, mut req: Request<Incoming>) -> Response<BoxBody<Bytes, std::io::Error>> {
    let url = req.uri().clone();
    match {
        let state = state.clone();
        async move {
            if (|| false)() {
                return Err(loga::err("")).err_internal() as Result<_, VisErr<loga::Error>>;
            }
            let (head, body) = req.into_parts();
            let mut path_iter = head.uri.path().trim_matches('/').split('/');
            match path_iter.next().unwrap() {
                "s1" => match path_iter.next().unwrap() {
                    _ => {
                        todo!();
                    },
                },
                "c1" => {
                    if hyper_tungstenite::is_upgrade_request(&req) {
                        // Websocket req
                        let upgrade = hyper_tungstenite::upgrade(&mut req, None);
                        let (head, _) = req.into_parts();
                        return Ok(response_503());
                        //. return Ok(handle_ws(state, head, upgrade, handle_ws_link).await);
                    } else {
                        let identity = identify_c2s(&state, &head.headers).await?;
                        match path_iter.next().unwrap() {
                            "client" => {
                                let path = head.uri.path().trim_matches('/');

                                #[derive(RustEmbed)]
                                #[folder = "$STATIC_DIR"]
                                struct Static;

                                let mut f = Static::get(path);
                                if f.is_none() {
                                    f = Static::get("index.html");
                                }
                                match f {
                                    Some(f) => {
                                        let etag = format!("\"{}\"", hex::encode(f.metadata.sha256_hash()));
                                        if let Some(h) = head.headers.get(IF_NONE_MATCH) {
                                            if h == etag.as_bytes() {
                                                return Ok(
                                                    Response::builder().status(304).body(body_full(vec![])).unwrap(),
                                                );
                                            }
                                        }
                                        let mut resp = Response::builder().status(200);
                                        resp = resp.header("Content-type", f.metadata.mimetype());
                                        resp = resp.header(ETAG, etag);
                                        return Ok(resp.body(body_full(f.data.to_vec())).unwrap());
                                    },
                                    None => {
                                        return Ok(response_404());
                                    },
                                }
                            },
                            "login" => {
                                return Ok(oidc::handle_oidc(&state.oidc_state, head).await?);
                            },
                            "post" => {
                                let Some(acc) = identify_c2s(&state, &head.headers).await? else {
                                    return Ok(response_401());
                                };
                                let resp;
                                match serde_json::from_slice::<c2s::proto::Req>(
                                    &body.collect().await.err_external()?.to_bytes(),
                                )
                                    .err_external()?
                                    .to_server_req() {
                                    c2s::proto::ServerReq::Logout(rr, r2) => {
                                        oidc::handle_logout(&state.oidc_state, &state.log, head).await;
                                        resp = rr(());
                                    },
                                    //.                                    c2s::proto::ServerReq::IdentityCreate(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    //.                                    c2s::proto::ServerReq::IdentityModify(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    //.                                    c2s::proto::ServerReq::IdentityDelete(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    //.                                    c2s::proto::ServerReq::IdentityGet(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    //.                                    c2s::proto::ServerReq::IdentityList(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    //.                                    c2s::proto::ServerReq::ChannelCreate(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    //.                                    c2s::proto::ServerReq::ChannelJoin(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    //.                                    c2s::proto::ServerReq::ChannelModify(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    //.                                    c2s::proto::ServerReq::ChannelDelete(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    //.                                    c2s::proto::ServerReq::ChannelGet(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    //.                                    c2s::proto::ServerReq::ChannelGroupCreate(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    //.                                    c2s::proto::ServerReq::ChannelGroupModify(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    //.                                    c2s::proto::ServerReq::ChannelGroupDelete(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    //.                                    c2s::proto::ServerReq::ChannelGroupGet(rr, r2) => {
                                    //.                                        resp = rr(());
                                    //.                                    },
                                    c2s::proto::ServerReq::ChannelOrChannelGroupTree(rr, channel_or_channel_group_tree) => {
                                        let (channels, channelgroups) = tx(&state.db, |conn| {
                                            return Ok(
                                                (
                                                    db::channel_list(conn, account),
                                                    db::channelgroup_list(conn, account),
                                                ),
                                            );
                                        }).await.err_internal()?;
                                        let mut out = vec![];
                                        let mut channelgroup_children = HashMap::new();
                                        for channel in channels.err_internal()? {
                                            let channel1 = ChannelRes {
                                                identity: channel.identity,
                                                id: channel.id,
                                                idem: channel.idem,
                                                memo_short: channel.memo_short,
                                                memo_long: channel.memo_long,
                                                group: channel.channel_group.clone(),
                                            };
                                            if let Some(group) = channel.channel_group {
                                                channelgroup_children
                                                    .entry(group.0.clone())
                                                    .or_default()
                                                    .push(channel1);
                                            } else {
                                                out.push(ChannelOrChannelGroup::Channel(channel1));
                                            }
                                        }
                                        for cg in channelgroups.err_internal()? {
                                            out.push(ChannelOrChannelGroup::ChannelGroup(ChannelOrChannelGroupGroup {
                                                group: ChannelGroupRes {
                                                    id: cg.rowid,
                                                    idem: cg.idem,
                                                    memo_short: cg.memo_short,
                                                    memo_long: cg.memo_long,
                                                },
                                                children: channelgroup_children
                                                    .remove(cg.rowid.clone())
                                                    .unwrap_or_default(),
                                            }));
                                        }
                                        resp = rr(out);
                                    },
                                //.                                    c2s::proto::ServerReq::IdentityInvitationCreate(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::IdentityInvitationModify(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::IdentityInvitationDelete(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::IdentityInvitationList(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::ChannelInvitationCreate(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::ChannelInvitationModify(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::ChannelInvitationDelete(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::ChannelInvitationList(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::MemberAdd(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::MemberDelete(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::MemberList(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::MessagePush(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::MessageLastPage(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::MessagePageContaining(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::MessageGetPage(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                //.                                    c2s::proto::ServerReq::MessageDelete(rr, r2) => {
                                //.                                        resp = rr(());
                                //.                                    },
                                }
                                return Ok(Response::builder().status(200).body(body_full(resp.0)).unwrap());
                            },
                        }
                    }
                },
            }
        }
    }.await {
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
    let runtime = runtime::Builder::new_current_thread().build().unwrap();
    match runtime.block_on({
        let log = log.clone();
        async move {
            ta_return!((), loga::Error);
            let tm = TaskManager::new();
            let args = vark::<Args>();
            let config = args.config.value;
            create_dirs(&config.persistent_dir).await?;
            let db_path = config.persistent_dir.join("db.sqlite3");

            // Spagh
            let spagh_node =
                spaghettinuum_native::service::node::Node::new(
                    &log,
                    &tm,
                    config.spagh_node_bind_sockaddr,
                    &spaghettinuum_native::service::node::default_bootstrap(),
                    &config.cache_dir,
                ).await?;
            let spagh_publisher_bind_sockaddr = config.spagh_publisher_bind_sockaddr.resolve()?;
            let spagh_publisher =
                spaghettinuum_native::service::publisher::Publisher::new(
                    &log,
                    &tm,
                    spagh_node.clone(),
                    spagh_publisher_bind_sockaddr,
                    SocketAddr::new(
                        resolve_global_ip(&log, &config.spagh_publisher_advertise_global_addr).await?.into(),
                        config.spagh_publisher_advertise_global_port.unwrap_or(spagh_publisher_bind_sockaddr.port()),
                    ),
                    &config.persistent_dir,
                ).await?;

            // Db
            let db =
                deadpool_sqlite::Config::new(&db_path)
                    .builder(deadpool_sqlite::Runtime::Tokio1)
                    .context("Error creating sqlite pool builder")?
                    .build()
                    .context("Error creating sqlite pool")?;
            db.get().await?.interact(move |conn| -> Result<_, loga::Error> {
                db::migrate(conn)?;
                return Ok(());
            }).await?.context_with("Migration failed", ea!(action = "db_init", path = db_path.to_string_lossy()))?;

            // State
            let state = Arc::new(State {
                log: log.clone(),
                db: db,
                oidc_state: oidc::new_state(&log, config.oidc_config).await?,
            });

            // Serve
            const S2SV1_PREFIX: &str = "/s/1";
            const C2SV1_PREFIX: &str = "/c/1";
            tm.critical_stream(
                "http",
                TcpListenerStream::new(
                    TcpListener::bind(config.bind_sockaddr.resolve().context("Error resolving server bind addr")?)
                        .await
                        .context_with("Error binding to address", ea!(addr = config.bind_sockaddr))?,
                ),
                {
                    let mut routes = BTreeMap::<String, Box<dyn Handler<Body>>>::new();

                    fn path_unshift<
                        'a,
                        E: std::error::Error,
                        T: FromStr<Err = E>,
                    >(path: &'a str, error_hint: &str) -> Result<(T, &'a str), loga::Error> {
                        let Some(r) = path.strip_prefix("/") else {
                            return Err(loga::err(format!("Missing path segment [{}]", error_hint)));
                        };
                        let next_slash = match r.find("/") {
                            Some(l) => l,
                            None => r.len(),
                        };
                        let seg =
                            T::from_str(
                                &r[0 .. next_slash],
                            ).context(format!("Error parsing path segment [{}]", error_hint))?;
                        let remainder = &r[next_slash..];
                        return Ok((seg, remainder));
                    }

                    move |conn| {
                        let state = state.clone();
                        async move {
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
                                        return Ok(handle_req(&state, req).await) as Result<_, std::io::Error>;
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
                        }
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
