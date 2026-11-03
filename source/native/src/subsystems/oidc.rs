use {
    crate::{
        cap_fn,
        db,
        dbutil::tx,
        interface::{
            config::OidcConfig,
            AccountExternalId,
        },
    },
    cookie::{
        CookieBuilder,
        SameSite,
    },
    deadpool_sqlite::Pool,
    http::{
        header::HOST,
        request::Parts,
        HeaderMap,
        Request,
        Response,
        Uri,
    },
    htwrap::{
        htreq,
        htserve::{
            responses::{
                body_empty,
                body_full,
                response_400,
                Body,
            },
            viserr::{
                ResultVisErr,
                VisErr,
            },
        },
        url::{
            new_abs_url,
            UriJoin,
        },
    },
    loga::{
        ea,
        ErrContext,
        Log,
        ResultContext,
    },
    moka::future::Cache,
    openidconnect::{
        core::{
            CoreAuthenticationFlow,
            CoreClient,
            CoreProviderMetadata,
        },
        AccessTokenHash,
        AuthorizationCode,
        ClientId,
        ClientSecret,
        CsrfToken,
        EndpointMaybeSet,
        EndpointNotSet,
        EndpointSet,
        IssuerUrl,
        Nonce,
        OAuth2TokenResponse,
        PkceCodeChallenge,
        PkceCodeVerifier,
        RedirectUrl,
        TokenResponse,
    },
    rand::distr::{
        Alphanumeric,
        SampleString,
    },
    serde::Deserialize,
    std::{
        sync::{
            Arc,
            Mutex,
        },
        time::Duration,
    },
};

pub const COOKIE_SESSION: &str = "kwa_session";
const PRE_SESSION_CAPACITY: u64 = 10_000;
const SESSION_IDLE_TIMEOUT: jiff::SignedDuration = jiff::SignedDuration::from_hours(24 * 7);
const SESSION_TOUCH_INTERVAL: jiff::SignedDuration = jiff::SignedDuration::from_hours(1);

async fn oidc_http_client(
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

struct OidcPreSession {
    pkce_verifier: Mutex<Option<PkceCodeVerifier>>,
    nonce: Nonce,
}

pub struct OidcState {
    log: loga::Log,
    public_url: Uri,
    client: CoreClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointMaybeSet, EndpointMaybeSet>,
    pre_sessions: Cache<String, Arc<OidcPreSession>>,
}

pub async fn new_state(log: &Log, public_url: &str, oidc_config: OidcConfig) -> Result<OidcState, loga::Error> {
    let log = log.fork(ea!(subsystem = "oidc"));
    let public_url = new_abs_url(public_url).context("Invalid public url")?;
    let redirect_url =
        RedirectUrl::new(
            public_url.join("/oidc").context("Error building OIDC redirect url from public url")?.to_string(),
        ).context("Invalid OIDC redirect url")?;
    let client =
        CoreClient::from_provider_metadata(
            CoreProviderMetadata::discover_async(IssuerUrl::new(oidc_config.provider_url)?, &cap_fn!((r)(log) {
                return oidc_http_client(&log, r).await.map_err(|e| std::io::Error::other(e.to_string()));
            })).await?,
            ClientId::new(oidc_config.client_id.clone()),
            oidc_config.client_secret.as_ref().map(|s| ClientSecret::new(s.clone())),
        ).set_redirect_uri(redirect_url);
    return Ok(OidcState {
        log: log,
        public_url: public_url,
        client: client,
        pre_sessions: Cache::builder()
            .max_capacity(PRE_SESSION_CAPACITY)
            .time_to_live(Duration::from_secs(60 * 10))
            .build(),
    });
}

pub async fn handle_oidc(
    state: &OidcState,
    db: &Pool,
    head: Parts,
) -> Result<Response<Body>, VisErr<loga::Error>> {
    let log = state.log.clone();

    #[derive(Deserialize)]
    struct CallbackParams {
        code: String,
        state: String,
    }

    if let Some(params) = head.uri.query().and_then(|q| serde_urlencoded::from_str::<CallbackParams>(q).ok()) {
        let Some(pre_session_state) = state.pre_sessions.remove(&params.state).await else {
            log.log_with(loga::DEBUG, "Missing pre-session state for state", ea!(state = params.state));
            return Ok(response_400("Login attempt expired or unknown"));
        };
        let Some(pkce_verifier) = pre_session_state.pkce_verifier.lock().unwrap().take() else {
            return Ok(response_400("Login attempt already used"));
        };
        let token_response =
            state
                .client
                .exchange_code(AuthorizationCode::new(params.code))
                .context("OIDC provider has no token endpoint")
                .err_internal()?
                .set_pkce_verifier(pkce_verifier)
                .request_async(&cap_fn!((r)(log) {
                    return oidc_http_client(&log, r).await.map_err(|e| std::io::Error::other(e.to_string()));
                }))
                .await
                .context("Error exchanging token from callback with OIDC server")
                .err_internal()?;
        let id_token = token_response.id_token().context("OIDC server response missing ID token").err_internal()?;
        let claims =
            id_token
                .claims(&state.client.id_token_verifier(), &pre_session_state.nonce)
                .context("Error getting claims from OIDC server response")
                .err_internal()?;
        if let Some(expected_access_token_hash) = claims.access_token_hash() {
            let actual_access_token_hash =
                AccessTokenHash::from_token(
                    token_response.access_token(),
                    id_token.signing_alg().context("Error getting signing alg from server response").err_internal()?,
                    id_token
                        .signing_key(&state.client.id_token_verifier())
                        .context("Error getting signing key from server response")
                        .err_internal()?,
                )
                    .context("Error hashing access token in server response")
                    .err_internal()?;
            if actual_access_token_hash != *expected_access_token_hash {
                log.log_with(
                    loga::DEBUG,
                    "Access token hash mismatch",
                    ea!(want = *expected_access_token_hash, got = actual_access_token_hash),
                );
                return Ok(response_400("Access token hash mismatch"));
            }
        }
        let session_cookie = Alphanumeric.sample_string(&mut rand::rng(), 32);
        session_create(db, session_cookie.clone(), AccountExternalId(claims.subject().to_string()))
            .await
            .err_internal()?;
        return Ok(
            http::Response::builder()
                .status(http::StatusCode::TEMPORARY_REDIRECT)
                .header(
                    http::header::SET_COOKIE,
                    CookieBuilder::new(COOKIE_SESSION, session_cookie)
                        .http_only(true)
                        .secure(true)
                        .same_site(SameSite::Lax)
                        .permanent()
                        .build()
                        .to_string(),
                )
                .header(http::header::LOCATION, state.public_url.to_string())
                .body(body_empty())
                .unwrap(),
        );
    }

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf_token, nonce) =
        state
            .client
            .authorize_url(CoreAuthenticationFlow::AuthorizationCode, CsrfToken::new_random, Nonce::new_random)
            .set_pkce_challenge(pkce_challenge)
            .url();
    state.pre_sessions.insert(csrf_token.secret().clone(), Arc::new(OidcPreSession {
        pkce_verifier: Mutex::new(Some(pkce_verifier)),
        nonce: nonce,
    })).await;
    return Ok(
        http::Response::builder()
            .status(http::StatusCode::TEMPORARY_REDIRECT)
            .header(http::header::LOCATION, auth_url.to_string())
            .body(body_empty())
            .unwrap(),
    );
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

async fn session_create(db: &Pool, session: String, external_id: AccountExternalId) -> Result<(), loga::Error> {
    let now = jiff::Timestamp::now();
    tx(db, move |db_tx| {
        good_ormning::sqlite::good_query!(
            db,
            r#"insert into session (id, external_id, last_used) values (${string = session.as_str()}, ${string = external_id.0.as_str()}, ${time_ms_t = &crate::interface::db::DbTimeMs(now)})"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(());
    }).await.context("Error storing session")?;
    return Ok(());
}

pub async fn session_lookup(db: &Pool, session: String) -> Result<Option<AccountExternalId>, loga::Error> {
    let now = jiff::Timestamp::now();
    let idle_cutoff = now.checked_sub(SESSION_IDLE_TIMEOUT).context("Session idle cutoff out of range")?;
    let touch_cutoff = now.checked_sub(SESSION_TOUCH_INTERVAL).context("Session touch cutoff out of range")?;
    return Ok(tx(db, move |db_tx| {
        let Some(row) =
            good_ormning::sqlite::good_query_opt!(
                db,
                r#"select external_id, last_used from session where id = ${string = session.as_str()}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))? else {
                return Ok(None);
            };
        if row.last_used.0 < idle_cutoff {
            good_ormning::sqlite::good_query!(
                db,
                r#"delete from session where id = ${string = session.as_str()}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
            return Ok(None);
        }
        if row.last_used.0 < touch_cutoff {
            good_ormning::sqlite::good_query!(
                db,
                r#"update session set last_used = ${time_ms_t = &crate::interface::db::DbTimeMs(now)} where id = ${string = session.as_str()}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        return Ok(Some(AccountExternalId(row.external_id)));
    }).await.context("Error looking up session")?);
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

pub async fn handle_logout(
    state: &OidcState,
    db: &Pool,
    head: Parts,
) -> Result<Response<Body>, VisErr<loga::Error>> {
    delete_req_session(db, &state.log, &head.headers).await.err_internal()?;
    return Ok(
        http::Response::builder()
            .status(http::StatusCode::TEMPORARY_REDIRECT)
            .header(http::header::LOCATION, state.public_url.to_string())
            .body(body_empty())
            .unwrap(),
    );
}
