use {
    crate::{
        db,
        interface::{
            db::DbIdentity,
            s2s::{
                Identify,
                IdentifyChallenge,
                IdentifyRes,
                StartIdentify,
                StartIdentifyRes,
            },
        },
    },
    crate::subsystems::s2s_error::{
        s2s_tx,
        S2sError,
    },
    deadpool_sqlite::Pool,
    loga::ResultContext,
    moka::future::Cache,
    rand::Rng,
    spaghettinuum::{
        byteszb32::BytesZb32,
        interface::identity::Identity,
    },
    spaghettinuum_native::service::resolver::Resolver,
    std::time::Duration,
};

pub const CHALLENGE_TTL: Duration = Duration::from_secs(30);
pub const TOKEN_TTL: Duration = Duration::from_secs(300);
const CHALLENGE_CAPACITY: u64 = 65536;
const TOKEN_CAPACITY: u64 = 65536;

pub struct S2sState {
    resolver: Resolver,
    challenges: Cache<Vec<u8>, Identity>,
    tokens: Cache<String, Identity>,
}

impl S2sState {
    pub fn new(resolver: Resolver) -> Self {
        return Self {
            resolver: resolver,
            challenges: Cache::builder().time_to_live(CHALLENGE_TTL).max_capacity(CHALLENGE_CAPACITY).build(),
            tokens: Cache::builder().time_to_live(TOKEN_TTL).max_capacity(TOKEN_CAPACITY).build(),
        };
    }

    pub async fn start_identify(&self, req: StartIdentify) -> StartIdentifyRes {
        let mut buf = [0u8; 32];
        rand::rng().fill_bytes(&mut buf);
        self.challenges.insert(buf.to_vec(), req.identity).await;
        return StartIdentifyRes { challenge: BytesZb32(buf.to_vec()) };
    }

    pub async fn identify(&self, db: &Pool, req: Identify) -> Result<IdentifyRes, S2sError> {
        let unverified = match serde_json::from_str::<IdentifyChallenge>(&req.challenge.message) {
            Ok(v) => v,
            Err(e) => return Err(S2sError::BadRequest(format!("Identify challenge body is not valid JSON: {}", e))),
        };
        let Some(expected_identity) = self.challenges.remove(&unverified.challenge.0).await else {
            return Err(S2sError::Unauthorized("No pending challenge matching response".to_string()));
        };
        if expected_identity != req.identity {
            return Err(S2sError::Unauthorized("Challenge was issued to a different identity".to_string()));
        }
        let db_audience = DbIdentity(unverified.audience);
        let audience_hosted = s2s_tx(db, "checking identify audience", move |db_tx| {
            return Ok(
                good_ormning::sqlite::good_query_opt!(
                    db,
                    r#"select 1 as x from identity where id = ${identity_id_t = &db_audience} and soft_deleted_at is null"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?,
            );
        }).await?;
        if audience_hosted.is_none() {
            return Err(S2sError::NotFound("Identify audience is not hosted on this server".to_string()));
        }
        let Some(kvs) =
            self.resolver.get(&req.identity, vec![]).await.context("Error resolving identity announcement")? else {
                return Err(S2sError::BadRequest("Identity has no announcement".to_string()));
            };
        let received =
            match req
                .challenge
                .verify(&req.identity, &kvs.announcement.public_key_root, &kvs.announcement.public_key_chain) {
                Ok(v) => v,
                Err(e) => return Err(S2sError::Unauthorized(format!("Signature verification failed: {}", e))),
            };
        if received.challenge.0 != unverified.challenge.0 || received.audience != unverified.audience {
            return Err(S2sError::BadRequest("Verified challenge doesn't match parsed challenge".to_string()));
        }
        let mut token_buf = [0u8; 32];
        rand::rng().fill_bytes(&mut token_buf);
        let token = hex::encode(&token_buf);
        self.tokens.insert(token.clone(), req.identity).await;
        return Ok(IdentifyRes { token: token });
    }

    pub async fn resolve_token(&self, token: &str) -> Option<Identity> {
        return self.tokens.get(token).await;
    }

    pub fn resolver(&self) -> &Resolver {
        return &self.resolver;
    }
}
