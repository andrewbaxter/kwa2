use {
    crate::{
        db,
        dbutil::tx,
    },
    deadpool_sqlite::Pool,
    http::{
        header::HOST,
        HeaderMap,
        Request,
        Uri,
    },
    htwrap::{
        htreq,
        htserve::responses::body_full,
    },
    loga::{
        ErrContext,
        Log,
        ResultContext,
    },
    moka::future::Cache,
    openidconnect::{
        core::CoreClient,
        EndpointMaybeSet,
        EndpointNotSet,
        EndpointSet,
        Nonce,
        PkceCodeVerifier,
    },
    std::sync::{
        Arc,
        Mutex,
    },
};

pub const COOKIE_SESSION: &str = "kwa_session";

pub async fn oidc_http_client(
    log: &loga::Log,
    req: openidconnect::HttpRequest,
) -> Result<openidconnect::HttpResponse, loga::Error> {
    let (mut parts, body) = req.into_parts();
    let mut conn = htreq::connect(htreq::Limits::default(), &parts.uri).await?;
    let authority = parts.uri.authority().context("OIDC request URL has no host")?.as_str().to_string();
    parts.headers.insert(HOST, http::HeaderValue::from_str(&authority).context("Invalid OIDC request host")?);
    let (code, headers, continue_) =
        htreq::send(log, htreq::Limits::default(), &mut conn, Request::from_parts(parts, body_full(body))).await?;
    let body = htreq::receive(htreq::Limits::default(), continue_).await?;
    let mut resp = openidconnect::HttpResponse::new(body);
    *resp.status_mut() = code;
    *resp.headers_mut() = headers;
    return Ok(resp);
}

pub struct OidcPreSession {
    pub pkce_verifier: Mutex<Option<PkceCodeVerifier>>,
    pub nonce: Nonce,
}

pub struct OidcState {
    pub log: loga::Log,
    pub public_url: Uri,
    pub client: CoreClient<
        EndpointSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointMaybeSet,
        EndpointMaybeSet,
    >,
    pub pre_sessions: Cache<String, Arc<OidcPreSession>>,
}

pub fn get_req_session<'a>(log: &Log, headers: &HeaderMap) -> Option<String> {
    let Some(v) = headers.get(http::header::COOKIE).and_then(|c| c.to_str().ok()) else {
        return None;
    };
    for cookie in cookie::Cookie::split_parse(v) {
        let cookie = match cookie {
            Ok(c) => c,
            Err(e) => {
                log.log_err(loga::DEBUG, e.context("Error parsing received cookie"));
                continue;
            },
        };
        if cookie.name() == COOKIE_SESSION {
            return Some(cookie.value().to_string());
        }
    }
    return None;
}

pub async fn delete_req_session(db: &Pool, log: &Log, headers: &HeaderMap) -> Result<(), loga::Error> {
    let Some(session) = get_req_session(log, headers) else {
        return Ok(());
    };
    tx(db, move |db_tx| {
        good_ormning::sqlite::good_query!(
            db,
            r#"delete from session where id = ${string = session.as_str()}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(());
    }).await.context("Error deleting session")?;
    return Ok(());
}
