use {
    crate::interface::s2s::{
        s2sv1,
        s2sv1t::NotifyTimestamp,
        ChannelInfoRes,
        GetActivity,
        GetChannelInfo,
        GetFile,
        GetJoinPreview,
        JoinPreviewRes,
        GetSnapLookup,
        GetSnapPage,
        GetSnapPageContainingTime,
        Identify,
        IdentifyChallenge,
        IdentifyRes,
        Notify,
        PullOutbox,
        PulledMessages,
        S2sErrorBody,
        SpaghRecord,
        SPAGH_RECORD_KEY,
        SnapLookupRes,
        StartIdentify,
        StartIdentifyRes,
    },
    http::{
        header::CONTENT_TYPE,
        Method,
        Request,
        Uri,
    },
    http_body_util::Full,
    htwrap::htreq::{
        self,
        Conn,
        Host,
        Ips,
        Limits,
    },
    hyper::body::Bytes,
    loga::{
        ea,
        Log,
        ResultContext,
    },
    moka::future::Cache,
    serde::de::DeserializeOwned,
    shared::interface::wire::c2s::PathReqTrait,
    spaghettinuum::{
        byteszb32::BytesZb32,
        interface::identity::Identity,
        jsonsig::JsonSignature,
        sigdomain::SigDomained,
    },
    spaghettinuum_native::{
        interface::{
            stored::record::tls_record,
            wire::resolve::DNS_DOT_SUFFIX,
        },
        service::resolver::Resolver,
        utils::{
            identity_secret::IdentitySigner,
            tls_util::{
                cert_pem_hash,
                client_config_builder,
                SpaghTlsClientVerifier,
            },
        },
    },
    std::{
        collections::{
            HashMap,
            HashSet,
        },
        str::FromStr,
        sync::{
            Arc,
            Mutex,
        },
        time::Duration,
    },
};

pub const S2SV1_PATH: &str = "/s/1";
const TOKEN_CACHE_TTL: Duration = Duration::from_secs(240);

pub type Signer = Arc<Mutex<dyn IdentitySigner>>;

pub struct S2sClient {
    log: Log,
    resolver: Resolver,
    limits: Limits,
    tokens: Cache<(Identity, Identity), String>,
}

fn auth_headers(token: &str) -> HashMap<String, String> {
    let mut h = HashMap::new();
    h.insert("Authorization".to_string(), format!("Bearer {}", token));
    return h;
}

fn signer_identity(signer: &Signer) -> Result<Identity, loga::Error> {
    let mut s = signer.lock().unwrap();
    return Ok(s.public_key_root()?.identity());
}

impl S2sClient {
    pub fn new(log: Log, resolver: Resolver) -> Self {
        return Self {
            log: log,
            resolver: resolver,
            limits: Limits {
                read_body_size: 16 * 1024 * 1024,
                ..Default::default()
            },
            tokens: Cache::builder().time_to_live(TOKEN_CACHE_TTL).build(),
        };
    }

    async fn connect(&self, target: Identity, path: &str) -> Result<(Conn, Uri), loga::Error> {
        let key_record = vec![SPAGH_RECORD_KEY.to_string()];
        let key_tls = vec![tls_record::KEY_SUFFIX_TLS.to_string()];
        let mut found =
            self
                .resolver
                .get(&target, vec![key_record.clone(), key_tls.clone()])
                .await
                .context_with("Error resolving peer identity", ea!(target = target))?
                .context_with("Peer identity has no announcement", ea!(target = target))?;
        let SpaghRecord::V1(record) =
            serde_json::from_value::<SpaghRecord>(
                found
                    .values
                    .remove(&key_record)
                    .and_then(|v| v.data)
                    .context_with("Peer identity has no kwa record", ea!(target = target))?,
            ).context_with("Peer identity kwa record is invalid", ea!(target = target))?;
        let tls_record::TlsCerts::V1(certs) =
            serde_json::from_value::<tls_record::TlsCerts>(
                found
                    .values
                    .remove(&key_tls)
                    .and_then(|v| v.data)
                    .context_with("Peer identity has no TLS record", ea!(target = target))?,
            ).context_with("Peer identity TLS record is invalid", ea!(target = target))?;
        let mut cert_hashes = HashSet::new();
        for cert in certs.0 {
            cert_hashes.insert(
                cert_pem_hash(&cert).context_with("Peer identity has invalid TLS cert", ea!(target = target))?,
            );
        }
        let mut ips = Ips {
            ipv4s: vec![],
            ipv6s: vec![],
        };
        for ip in record.ips {
            ips.push(ip);
        }
        let host = format!("{}{}", target, DNS_DOT_SUFFIX);
        let url = Uri::from_str(&format!("https://{}:{}{}{}", host, record.port, S2SV1_PATH, path)).unwrap();
        let conn =
            htreq::connect_ips(
                self.limits,
                ips,
                client_config_builder()
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(SpaghTlsClientVerifier {
                        hashes: cert_hashes,
                        inner: None,
                    }))
                    .with_no_client_auth(),
                "https".to_string(),
                Host::Name(host),
                record.port,
            )
                .await
                .context_with("Error connecting to peer", ea!(url = url))?;
        return Ok((conn, url));
    }

    async fn request<T: DeserializeOwned>(
        &self,
        target: Identity,
        method: Method,
        path: &str,
        headers: &HashMap<String, String>,
        body: Vec<u8>,
    ) -> Result<T, loga::Error> {
        let (_, body, url) = self.request_raw(target, method, path, headers, body).await?;
        return Ok(
            serde_json::from_slice::<T>(
                &body,
            ).context_with(
                "Error deserializing s2s response",
                ea!(url = url, body = String::from_utf8_lossy(&body)),
            )?,
        );
    }

    async fn request_raw(
        &self,
        target: Identity,
        method: Method,
        path: &str,
        headers: &HashMap<String, String>,
        body: Vec<u8>,
    ) -> Result<(String, Vec<u8>, Uri), loga::Error> {
        let (mut conn, url) = self.connect(target, path).await?;
        let mut builder = Request::builder().method(method).uri(url.clone());
        if !body.is_empty() {
            builder = builder.header(CONTENT_TYPE, "application/json");
        }
        for (k, v) in headers {
            builder = builder.header(k, v);
        }
        let request = builder.body(Full::new(Bytes::from(body))).unwrap();
        let (status, resp_headers, continue_send) =
            htreq::send(&self.log, self.limits, &mut conn, request)
                .await
                .context_with("Error sending s2s request", ea!(url = url))?;
        let body =
            htreq::receive(self.limits, continue_send)
                .await
                .context_with("Error reading s2s response", ea!(url = url))?;
        if !status.is_success() {
            let err_body =
                serde_json::from_slice::<S2sErrorBody>(
                    &body,
                ).context_with(
                    "Peer returned error with unparseable body",
                    ea!(url = url, status = status, body = String::from_utf8_lossy(&body)),
                )?;
            return Err(
                loga::err_with(
                    "Peer returned error",
                    ea!(url = url, status = status, kind = format!("{:?}", err_body.kind), message = err_body.message),
                ),
            );
        }
        let content_type =
            resp_headers
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .context_with("Peer response missing content type", ea!(url = url))?
                .to_string();
        return Ok((content_type, body, url));
    }

    async fn post<T: DeserializeOwned>(
        &self,
        target: Identity,
        headers: &HashMap<String, String>,
        req: &s2sv1::Req,
    ) -> Result<T, loga::Error> {
        return self.request(target, Method::POST, "", headers, serde_json::to_vec(req).unwrap()).await;
    }

    async fn get<R: PathReqTrait>(
        &self,
        target: Identity,
        headers: &HashMap<String, String>,
        req: &R,
    ) -> Result<R::Resp, loga::Error> {
        return self.request(target, Method::GET, &format!("/{}", req.serialize_path()), headers, vec![]).await;
    }

    async fn authenticate(&self, target: Identity, signer: &Signer) -> Result<String, loga::Error> {
        let identity = signer_identity(signer)?;
        if let Some(token) = self.tokens.get(&(target, identity)).await {
            return Ok(token);
        }
        let start: StartIdentifyRes =
            self
                .post(target, &HashMap::new(), &s2sv1::Req::StartIdentify(StartIdentify { identity: identity }))
                .await?;
        let message = serde_json::to_string(&IdentifyChallenge {
            challenge: start.challenge,
            audience: target,
        }).unwrap();
        let (_root, _chain, sig) = signer.lock().unwrap().sign(IdentifyChallenge::DOMAIN, message.as_bytes())?;
        let signed = JsonSignature {
            message: message,
            signature: BytesZb32(sig),
            _p: Default::default(),
        };
        let identify: IdentifyRes =
            self.post(target, &HashMap::new(), &s2sv1::Req::Identify(Identify {
                identity: identity,
                challenge: signed,
            })).await?;
        self.tokens.insert((target, identity), identify.token.clone()).await;
        return Ok(identify.token);
    }

    async fn post_authed<T: DeserializeOwned>(
        &self,
        target: Identity,
        signer: &Signer,
        req: &s2sv1::Req,
    ) -> Result<T, loga::Error> {
        let identity = signer_identity(signer)?;
        let token = self.authenticate(target, signer).await?;
        let res = self.post(target, &auth_headers(&token), req).await;
        if res.is_err() {
            self.tokens.invalidate(&(target, identity)).await;
        }
        return res;
    }

    async fn get_opt_authed<R: PathReqTrait>(
        &self,
        target: Identity,
        signer: Option<&Signer>,
        req: &R,
    ) -> Result<R::Resp, loga::Error> {
        let Some(signer) = signer else {
            return self.get(target, &HashMap::new(), req).await;
        };
        let identity = signer_identity(signer)?;
        let token = self.authenticate(target, signer).await?;
        let res = self.get(target, &auth_headers(&token), req).await;
        if res.is_err() {
            self.tokens.invalidate(&(target, identity)).await;
        }
        return res;
    }

    pub async fn notify(
        &self,
        target: Identity,
        signer: &Signer,
        owner: Identity,
        channel: shared::interface::shared::ChannelId,
    ) -> Result<(), loga::Error> {
        let ts = NotifyTimestamp {
            owner: owner,
            channel: channel,
            timestamp: jiff::Timestamp::now(),
        };
        let message = serde_json::to_string(&ts).unwrap();
        let (_root, _chain, sig) = signer.lock().unwrap().sign(NotifyTimestamp::DOMAIN, message.as_bytes())?;
        let signed = JsonSignature {
            message: message,
            signature: BytesZb32(sig),
            _p: Default::default(),
        };
        return self.post(target, &HashMap::new(), &s2sv1::Req::Notify(Notify {
            signer: signer_identity(signer)?,
            signed: signed,
        })).await;
    }

    pub async fn join(
        &self,
        target: Identity,
        signer: &Signer,
        req: crate::interface::s2s::Join,
    ) -> Result<crate::interface::s2s::JoinRes, loga::Error> {
        return self.post_authed(target, signer, &s2sv1::Req::Join(req)).await;
    }

    pub async fn pull_outbox(
        &self,
        target: Identity,
        signer: &Signer,
        req: PullOutbox,
    ) -> Result<PulledMessages, loga::Error> {
        return self.post_authed(target, signer, &s2sv1::Req::PullOutbox(req)).await;
    }

    pub async fn get_file(
        &self,
        target: Identity,
        signer: Option<&Signer>,
        req: GetFile,
    ) -> Result<(String, Vec<u8>), loga::Error> {
        let path = format!("/{}", req.serialize_path());
        let Some(signer) = signer else {
            let (mime, body, _) = self.request_raw(target, Method::GET, &path, &HashMap::new(), vec![]).await?;
            return Ok((mime, body));
        };
        let identity = signer_identity(signer)?;
        let token = self.authenticate(target, signer).await?;
        let res = self.request_raw(target, Method::GET, &path, &auth_headers(&token), vec![]).await;
        if res.is_err() {
            self.tokens.invalidate(&(target, identity)).await;
        }
        let (mime, body, _) = res?;
        return Ok((mime, body));
    }

    pub async fn get_join_preview(
        &self,
        target: Identity,
        req: GetJoinPreview,
    ) -> Result<JoinPreviewRes, loga::Error> {
        return self.get(target, &HashMap::new(), &req).await;
    }

    pub async fn get_channel_info(
        &self,
        target: Identity,
        signer: Option<&Signer>,
        req: GetChannelInfo,
    ) -> Result<ChannelInfoRes, loga::Error> {
        return self.get_opt_authed(target, signer, &req).await;
    }

    pub async fn get_activity(
        &self,
        target: Identity,
        signer: Option<&Signer>,
        req: GetActivity,
    ) -> Result<PulledMessages, loga::Error> {
        return self.get_opt_authed(target, signer, &req).await;
    }

    pub async fn get_snap_page(
        &self,
        target: Identity,
        signer: Option<&Signer>,
        req: GetSnapPage,
    ) -> Result<Option<PulledMessages>, loga::Error> {
        return self.get_opt_authed(target, signer, &req).await;
    }

    pub async fn get_snap_page_containing_time(
        &self,
        target: Identity,
        signer: Option<&Signer>,
        req: GetSnapPageContainingTime,
    ) -> Result<Option<u64>, loga::Error> {
        return self.get_opt_authed(target, signer, &req).await;
    }

    pub async fn get_snap_lookup(
        &self,
        target: Identity,
        signer: Option<&Signer>,
        req: GetSnapLookup,
    ) -> Result<Option<SnapLookupRes>, loga::Error> {
        return self.get_opt_authed(target, signer, &req).await;
    }
}
