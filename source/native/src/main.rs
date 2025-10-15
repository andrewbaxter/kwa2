pub mod interface;
pub mod dbutil;

use {
    crate::interface::shared::ChannelId,
    aargvark::{
        traits_impls::AargvarkJson,
        vark,
        Aargvark,
    },
    flowcontrol::{
        shed,
        ta_return,
    },
    glove::reqresp,
    http::{
        header::{
            ETAG,
            IF_NONE_MATCH,
        },
        Method,
        Response,
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
                response_404,
                Body,
            },
        },
    },
    loga::{
        ea,
        fatal,
        ErrContext,
        Log,
        ResultContext,
    },
    rust_embed::RustEmbed,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    spaghettinuum::{
        interface::{
            config::shared::{
                GlobalAddrConfig,
                StrSocketAddr,
            },
            stored::identity::Identity,
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
    pub data_dir: PathBuf,
}

#[derive(Aargvark)]
struct Args {
    config: AargvarkJson<Config>,
}

fn main() {
    let log = Log::new_root(loga::DEBUG);
    let (bg_tx, bg_rx) = mpsc::channel(100);
    let runtime = runtime::Builder::new_current_thread().on_thread_park({
        let log = log.clone();
        move || {
            if let Ok(task) = bg_rx.try_recv() {
                spawn(async move {
                    match task.await {
                        Ok(_) => { },
                        Err(e) => {
                            log.log_err(loga::WARN, e.context("Background task exited with error"));
                        },
                    }
                });
            }
        }
    }).build().unwrap();
    match runtime.block_on(async {
        ta_return!((), loga::Error);
        let tm = TaskManager::new();
        let args = vark::<Args>();
        let config = args.config.value;
        let spagh_node =
            spaghettinuum::service::node::Node::new(
                &log,
                &tm,
                config.spagh_node_bind_sockaddr,
                &spaghettinuum::service::node::default_bootstrap(),
                &config.cache_dir,
            ).await?;
        let spagh_publisher_bind_sockaddr = config.spagh_publisher_bind_sockaddr.resolve()?;
        let spagh_publisher =
            spaghettinuum::service::publisher::Publisher::new(
                &log,
                &tm,
                spagh_node.clone(),
                spagh_publisher_bind_sockaddr,
                SocketAddr::new(
                    resolve_global_ip(&log, &config.spagh_publisher_advertise_global_addr).await?.into(),
                    config.spagh_publisher_advertise_global_port.unwrap_or(spagh_publisher_bind_sockaddr.port()),
                ),
                &config.data_dir,
            ).await?;
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
                let mut routes = BTreeMap::new();

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

                // # s2s routes
                routes.insert(format!("{}/last_page", S2SV1_PREFIX), Box::new(handler!((state: Arc < State >)(req -> Body) {
                    let path = req.subpath;
                    let (owner_id, path) = path_unshift::<Identity>(path, "identity id")?;
                    let (channel_id, path) = path_unshift::<ChannelId>(path, "channel id")?;
                    return tx(state.db, |db| {
                        let Some(channel) = channel_get_owned(db, channel_id)? else {
                            return Ok(None);
                        };
                        if !channel.is_public {
                            let identity = match check_identity(req)? {
                                Ok(i) => i,
                                Err(e) => {
                                    return Ok(start_identify(state));
                                },
                            };
                            if !channel_get_members(identity) {
                                return Ok(None);
                            }
                        }
                        let count = message_get_page_count(db, channel_id)?;
                        return Ok(Some(ResLastPage {
                            page_size: PAGE_SIZE,
                            count: count,
                        }));
                    }).await?;
                })) as Box<dyn Handler<Body>>);
                routes.insert(
                    format!("{}/page_containing", S2SV1_PREFIX),
                    Box::new(handler!((state: Arc < State >)(req -> Body) {
                        let path = req.subpath;
                        let (owner_id, path) = path_unshift::<IdentityId>(path, "identity id")?;
                        let (channel_id, path) = path_unshift::<ChannelId>(path, "channel id")?;
                        let (message_id, _) = path_unshift::<MessageId>(path, "message id")?;
                        return tx(state.db, |db| {
                            let Some(channel) = channel_get_owned(db, channel_id)? else {
                                return Ok(None);
                            };
                            if !channel.is_public {
                                let identity = match check_identity(req)? {
                                    Ok(i) => i,
                                    Err(e) => {
                                        return Ok(start_identify(state));
                                    },
                                };
                                if !channel_get_members(identity) {
                                    return Ok(None);
                                }
                            }
                            let message = message_get(db, channel_id, message_id)?;
                            return Ok(Some(message.page));
                        }).await?;
                    })),
                );
                routes.insert(format!("{}/page", S2SV1_PREFIX), Box::new(handler!((state: Arc < State >)(req -> Body) {
                    let path = req.subpath;
                    let (owner_id, path) = path_unshift::<IdentityId>(path, "identity id")?;
                    let (channel_id, path) = path_unshift::<ChannelId>(path, "channel id")?;
                    let (page, _) = path_unshift::<usize>(path, "page")?;
                    return tx(state.db, |db| {
                        let Some(channel) = channel_get_owned(db, channel_id)? else {
                            return Ok(None);
                        };
                        if !channel.is_public {
                            let identity = match check_identity(req)? {
                                Ok(i) => i,
                                Err(e) => {
                                    return Ok(start_identify(state));
                                },
                            };
                            if !channel_get_members(identity) {
                                return Ok(None);
                            }
                        }
                        let messages = message_get_page(db, channel_id, page)?;
                        return Ok(Some(ResMessages {
                            values: messages,
                            page_size: PAGE_SIZE,
                            cache: messages.len() == PAGE_SIZE,
                        }));
                    }).await?;
                })));
                routes.insert(format!("{}/post", S2SV1_PREFIX), Box::new(handler!((state: Arc < State >)(req -> Body) {
                    let resp = shed!{
                        fn start_identify() {
                            let challenge = rng().generate_rand();
                            state.pending_identify.insert(challenge);
                            return Challenge(challenge);
                        }

                        match serde_json::from_slice::<s2sv1::Req>(&req.body.bytes().await?)?.to_server_req() {
                            s2sv1::ServerReq::StartIdentify(rr, r) => {
                                break rr(start_identify());
                            },
                            s2sv1::ServerReq::Identify(rr, r) => {
                                let Some(c) = state.pending_identify.remove(r.challenge.value) else {
                                    break rr(None);
                                };
                                let token = rng().generate_rand();
                                state.authorizations.insert(token.clone(), r.challenge.signer);
                                break rr(Some(token));
                            },
                            s2sv1::ServerReq::Notify(rr, r) => {
                                let Some(ident) = check_ident(req).await? else {
                                    break rr(start_identify());
                                };
                                if let Some(channel) = get_owned_channel(r.channel) {
                                    let do_pull = tx(db, |db| {
                                        if !channel_is_member(r.channel, ident) {
                                            return Ok(false);
                                        }
                                        let acc = owner_get_by_ident(r.channel.owner);
                                        return Ok(owned_channel_set_dirty(acc, r.channel, true));
                                    }).await?;
                                    if !do_pull {
                                        state
                                            .bg_tx
                                            .try_send(channel_pull(state.clone(), r.channel, ident))
                                            .ignore();
                                    }
                                    break rr(());
                                } else if ident == r.channel.owner {
                                    let dirty_channels = tx(db, |db| {
                                        channels_set_dirty(acc, r.channel, true)
                                    });
                                    for channel in dirty_channels {
                                        state
                                            .bg_tx
                                            .try_send(channel_pull(state.clone(), channel, channel.owner))
                                            .ignore();
                                    }
                                    break rr(());
                                } else {
                                    break rr(());
                                }
                            },
                            s2sv1::ServerReq::Sub(rr, r) => {
                                let Some(ident) = check_ident(req).await? else {
                                    break rr(start_identify());
                                };
                                let res = tx(db, |db| {
                                    let Some(permit) = get_sub_permit(r.token) else {
                                        return None;
                                    };
                                    if now().signed_duration_since(permit.expired).is_positive() {
                                        permit_remove(r.token)?;
                                        return None;
                                    }
                                    if let Some(permit_ident) = permit.who {
                                        if permit_ident != ident {
                                            return None;
                                        }
                                    }
                                    if permit.once {
                                        permit_remove(r.token)?;
                                    }
                                    let channel = match permit.specific {
                                        SubInvitation::User(p) => {
                                            create_channel(p.user, p.channel_name)
                                        },
                                        SubInvitation::Channel(p) => {
                                            p.channel
                                        },
                                    };
                                    channel_ensure_member(p.channel, ident)?;
                                    return Some(p.channel);
                                }).await?;
                                break rr(());
                            },
                        }
                    };
                    return Ok(response_200_json(resp));
                })));

                //.                routes.insert(format!("{}/file", S2S_PREFIX), Box::new(handler!((state: Arc < State >)(req -> Body) {
                //.                    let Some(ident) = check_ident(req).await? else {
                //.                        break rr(start_identify());
                //.                    };
                //.                    let (file_id, _) = path_unshift(req.subpath);
                //.                    let file =
                //.                        FileHash::from_str(file_id)
                //.                            .map_err(|e| loga::err(e).context_with("Couldn't parse hash", ea!(hash = file_id)))
                //.                            .err_external()?;
                //.                    let meta = tx(db, |db| {
                //.                        return Ok(check_file_access(db, ident, file)?);
                //.                    })?;
                //.                    if meta.is_none() {
                //.                        return Ok(start_identify());
                //.                    }
                //.                    match req.head.method {
                //.                        Method::HEAD => {
                //.                            return handle_file_head(state, file).await;
                //.                        },
                //.                        Method::GET => {
                //.                            return handle_file_get(state, head, file, gentype, subpath).await;
                //.                        },
                //.                        _ => return Ok(response_404()),
                //.                    }
                //.                })));
                // # c2s routes
                routes.insert(format!("{}/post", C2SV1_PREFIX), Box::new(handler!((state: Arc < State >)(req -> Body) {
                    let resp;
                    match serde_json::from_slice::<c2sv1::Req>(&req.body.bytes().await?)?.to_server_req() { }
                    return Ok(response_200_json(resp));
                })));

                //.                routes.insert(format!("{}/file", S2S_PREFIX), Box::new(handler!((state: Arc < State >)(req -> Body) {
                //.                    let Some(ident) = check_ident(req).await? else {
                //.                        break rr(start_identify());
                //.                    };
                //.                    let (file_id, _) = path_unshift(req.subpath);
                //.                    let file =
                //.                        FileHash::from_str(file_id)
                //.                            .map_err(|e| loga::err(e).context_with("Couldn't parse hash", ea!(hash = file_id)))
                //.                            .err_external()?;
                //.                    let meta = tx(db, |db| {
                //.                        return Ok(check_file_access(db, ident, file)?);
                //.                    })?;
                //.                    if meta.is_none() {
                //.                        return Ok(start_identify());
                //.                    }
                //.                    match req.head.method {
                //.                        Method::HEAD => {
                //.                            return handle_file_head(state, file).await;
                //.                        },
                //.                        Method::GET => {
                //.                            return handle_file_get(state, head, file, gentype, subpath).await;
                //.                        },
                //.                        _ => return Ok(response_404()),
                //.                    }
                //.                })));
                // web client
                routes.insert(format!("/client"), Box::new(handler!((state: Arc < State >)(req -> Body) {
                    let path = req.head.uri.path().trim_matches('/');

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
                            if let Some(h) = req.head.headers.get(IF_NONE_MATCH) {
                                if h == etag.as_bytes() {
                                    return Response::builder().status(304).body(body_full(vec![])).unwrap();
                                }
                            }
                            let mut resp = Response::builder().status(200);
                            for (k, v) in &state.http_resp_headers {
                                resp = resp.header(k, v);
                            }
                            resp = resp.header("Content-type", f.metadata.mimetype());
                            resp = resp.header(ETAG, etag);
                            return resp.body(body_full(f.data.to_vec())).unwrap();
                        },
                        None => {
                            return response_404();
                        },
                    }
                })));
                let routes =
                    Arc::new(
                        PathRouter::new(routes)
                            .map_err(
                                |e| loga::agg_err(
                                    "Error constructing request path router",
                                    e.into_iter().map(loga::err).collect(),
                                ),
                            )
                            .context("Error setting up s2s router")?,
                    );
                let log = log.clone();
                async move |conn| {
                    let conn = match conn {
                        Ok(c) => c,
                        Err(e) => {
                            log.log_err(loga::DEBUG, e.context("Error receiving request"));
                            return Ok(());
                        },
                    };
                    match htserve::handler::root_handle_http(&log, routes, conn).await {
                        Ok(_) => { },
                        Err(e) => {
                            log.log_err(loga::DEBUG, e.context("Error handling request"));
                        },
                    }
                    return Ok(());
                }
            },
        );
        tm.join(&log).await?;
        return Ok(());
    }) {
        Ok(_) => { },
        Err(e) => {
            fatal(e);
        },
    }
}
