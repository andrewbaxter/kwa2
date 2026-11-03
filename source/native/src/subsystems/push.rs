use {
    crate::{
        db,
        dbutil::tx,
        State,
    },
    base64::{
        engine::general_purpose::URL_SAFE_NO_PAD,
        Engine,
    },
    http::{
        header::HOST,
        Method,
        Request,
        StatusCode,
        Uri,
    },
    htwrap::htreq::{
        self,
        Limits,
    },
    loga::{
        ea,
        ErrContext,
        ResultContext,
    },
    p256::{
        elliptic_curve::{
            sec1::ToSec1Point,
            Generate,
        },
        SecretKey,
    },
    shared::interface::shared::AccountId,
    std::{
        collections::HashMap,
        str::FromStr,
        sync::Arc,
    },
    web_push::{
        ContentEncoding,
        SubscriptionInfo,
        VapidSignatureBuilder,
        WebPushMessageBuilder,
    },
};

const KEY_VAPID_PRIVATE_B64: &str = "vapid_private_b64";
const KEY_VAPID_PUBLIC_B64: &str = "vapid_public_b64";
const PUSH_LIMITS: Limits = Limits {
    read_body_size: 64 * 1024,
    resolve_time: std::time::Duration::from_secs(10),
    connect_time: std::time::Duration::from_secs(10),
    read_header_time: std::time::Duration::from_secs(30),
    read_body_time: std::time::Duration::from_secs(30),
    read_idle_time: std::time::Duration::from_secs(30),
};

pub struct VapidKeypair {
    pub private_b64url: String,
    pub public_b64url: String,
}

pub async fn ensure_vapid_keypair(pool: &deadpool_sqlite::Pool) -> Result<VapidKeypair, loga::Error> {
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
}

pub async fn send_push_to_account(
    state: &Arc<State>,
    account_id: AccountId,
    payload: &[u8],
) -> Result<(), loga::Error> {
    let account_id_i64 = i64::from_ne_bytes(account_id.0.to_ne_bytes());
    let subs = tx(&state.db, move |db_tx| {
        return good_ormning::sqlite::good_query_many!(
            db,
            r#"select data from push_subscription where account_id = ${i64 = account_id_i64}"#;
            db_tx
        ).map_err(|e| loga::err(e.0));
    }).await.context("Error listing push subscriptions")?;
    if subs.is_empty() {
        return Ok(());
    }
    let vapid = &state.vapid_keys;
    let mut expired_endpoints: Vec<String> = Vec::new();
    for data_json in subs {
        let info: SubscriptionInfo = match serde_json::from_str(&data_json) {
            Ok(v) => v,
            Err(e) => {
                state
                    .log
                    .log_err(
                        loga::WARN,
                        loga::err(e.to_string()).context("Skipping malformed push subscription record"),
                    );
                continue;
            },
        };
        let endpoint = info.endpoint.clone();
        let res = async {
            flowcontrol::ta_return!((), PushSendError);
            let vapid_sig =
                VapidSignatureBuilder::from_base64_no_sub(&vapid.private_b64url)
                    .map_err(|e| PushSendError::Other(loga::err(e.to_string()).context("VAPID key parse failed")))?
                    .add_sub_info(&info)
                    .build()
                    .map_err(
                        |e| PushSendError::Other(loga::err(e.to_string()).context("VAPID signature build failed")),
                    )?;
            let mut builder = WebPushMessageBuilder::new(&info);
            builder.set_payload(ContentEncoding::Aes128Gcm, payload);
            builder.set_vapid_signature(vapid_sig);
            let msg =
                builder
                    .build()
                    .map_err(
                        |e| PushSendError::Other(
                            loga::err(e.to_string()).context("Failed to build web-push message"),
                        ),
                    )?;
            let mut headers = HashMap::new();
            headers.insert("TTL".to_string(), msg.ttl.to_string());
            if let Some(u) = msg.urgency {
                headers.insert("Urgency".to_string(), u.to_string());
            }
            if let Some(t) = msg.topic {
                headers.insert("Topic".to_string(), t);
            }
            let body = if let Some(p) = msg.payload {
                headers.insert("Content-Encoding".to_string(), p.content_encoding.to_str().to_string());
                headers.insert("Content-Type".to_string(), "application/octet-stream".to_string());
                headers.insert("Content-Length".to_string(), p.content.len().to_string());
                for (k, v) in p.crypto_headers {
                    headers.insert(k.to_string(), v);
                }
                p.content
            } else {
                Vec::new()
            };
            let endpoint_uri: Uri =
                Uri::from_str(
                    &msg.endpoint.to_string(),
                ).map_err(
                    |e| PushSendError::Other(loga::err(e.to_string()).context("Invalid push endpoint URL")),
                )?;
            let authority =
                endpoint_uri
                    .authority()
                    .ok_or_else(|| PushSendError::Other(loga::err("Push endpoint URL has no authority")))?
                    .to_string();
            let mut conn: htreq::Conn =
                htreq::connect(PUSH_LIMITS, &endpoint_uri)
                    .await
                    .map_err(|e| PushSendError::Other(e.context("Failed to connect to push endpoint")))?;
            let mut req = Request::builder().method(Method::POST).uri(endpoint_uri.clone()).header(HOST, authority);
            for (k, v) in headers {
                req = req.header(k, v);
            }
            let req =
                req
                    .body(http_body_util::Full::new(hyper::body::Bytes::from(body)))
                    .map_err(
                        |e| PushSendError::Other(loga::err(e.to_string()).context("Error building push request")),
                    )?;
            let (status, _, continue_) =
                htreq::send(&state.log, PUSH_LIMITS, &mut conn, req)
                    .await
                    .map_err(|e| PushSendError::Other(e.context("Push endpoint POST failed")))?;
            if status == StatusCode::NOT_FOUND || status == StatusCode::GONE {
                return Err(PushSendError::Gone);
            }
            if !status.is_success() {
                let err = loga::err_with("Push endpoint returned error status", ea!(status = status));
                match htreq::receive(PUSH_LIMITS, continue_).await {
                    Ok(body) => {
                        return Err(
                            PushSendError::Other(
                                err.context_with("Error response body", ea!(body = String::from_utf8_lossy(&body))),
                            ),
                        );
                    },
                    Err(e) => {
                        return Err(
                            PushSendError::Other(err.also(e.context("Error reading error response body"))),
                        );
                    },
                }
            }
            return Ok(());
        }.await;
        match res {
            Ok(()) => { },
            Err(PushSendError::Gone) => {
                expired_endpoints.push(endpoint);
            },
            Err(PushSendError::Other(e)) => {
                state.log.log_err(loga::WARN, e.context_with("Push delivery failed", ea!(endpoint = endpoint)));
            },
        }
    }
    if !expired_endpoints.is_empty() {
        let expired = expired_endpoints;
        tx(&state.db, move |db_tx| {
            for e in &expired {
                let e = e.clone();
                good_ormning::sqlite::good_query!(
                    db,
                    r#"delete from push_subscription where account_id = ${i64 = account_id_i64} and endpoint = ${string = e.as_str()}"#;
                    db_tx
                ).map_err(|err| loga::err(err.0))?;
            }
            return Ok(());
        }).await.context("Error pruning expired push subscriptions")?;
    }
    return Ok(());
}

enum PushSendError {
    Gone,
    Other(loga::Error),
}
