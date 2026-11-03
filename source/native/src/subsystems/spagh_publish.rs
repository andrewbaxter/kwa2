use {
    crate::{
        db,
        dbutil::tx,
        interface::{
            db::DbIdentity,
            s2s::{
                SpaghRecord,
                SPAGH_RECORD_KEY,
            },
        },
        subsystems::messages::local_signer,
        State,
    },
    loga::{
        ea,
        ResultContext,
    },
    rustls::{
        server::{
            ClientHello,
            ResolvesServerCert,
        },
        sign::CertifiedKey,
    },
    spaghettinuum::interface::identity::Identity,
    spaghettinuum_native::{
        interface::{
            stored::{
                record::{
                    self,
                    tls_record,
                },
                self_tls::{
                    self as stored_self_tls,
                    RefreshTlsState,
                },
            },
            wire::{
                api::publish::latest::InfoResponse,
                resolve::DNS_DOT_SUFFIX,
            },
        },
        self_tls::{
            request_cert,
            RequestCertOptions,
        },
        service::publisher::Publisher,
        utils::{
            identity_secret::IdentitySigner,
            publish_util::{
                generate_publish_announce,
                PublishArgs,
            },
            tls_util::{
                extract_expiry,
                load_certified_key,
            },
        },
    },
    std::{
        collections::HashMap,
        str::FromStr,
        sync::{
            Arc,
            Mutex,
            RwLock,
        },
        time::{
            Duration,
            SystemTime,
        },
    },
};

const RECORD_TTL: Duration = Duration::from_secs(60 * 60);
const CERT_SWAP_BUFFER: Duration = Duration::from_secs(60 * 60 * 24 * 7);
pub const CERT_REFRESH_PERIOD: Duration = Duration::from_secs(60 * 60);

pub struct IdentityCerts(RwLock<HashMap<Identity, Arc<CertifiedKey>>>);

impl std::fmt::Debug for IdentityCerts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f.debug_list().entries(self.0.read().unwrap().keys()).finish();
    }
}

impl ResolvesServerCert for IdentityCerts {
    fn resolve(&self, client_hello: ClientHello) -> Option<Arc<CertifiedKey>> {
        let identity = Identity::from_str(client_hello.server_name()?.strip_suffix(DNS_DOT_SUFFIX)?).ok()?;
        return self.0.read().unwrap().get(&identity).cloned();
    }
}

pub struct SpaghPublishState {
    pub publisher: Arc<Publisher>,
    pub publisher_info: InfoResponse,
    pub record: serde_json::Value,
    pub certs: Arc<IdentityCerts>,
}

impl SpaghPublishState {
    pub fn new(publisher: Arc<Publisher>, publisher_info: InfoResponse, record: SpaghRecord) -> Self {
        return Self {
            publisher: publisher,
            publisher_info: publisher_info,
            record: serde_json::to_value(&record).unwrap(),
            certs: Arc::new(IdentityCerts(RwLock::new(HashMap::new()))),
        };
    }
}

fn cert_options() -> RequestCertOptions {
    return RequestCertOptions {
        certifier: None,
        signature: true,
    };
}

async fn store_tls_state(
    state: &State,
    identity: Identity,
    tls_state: stored_self_tls::latest::RefreshTlsState,
) -> Result<(), loga::Error> {
    let db_identity = DbIdentity(identity);
    let tls_state = RefreshTlsState::V1(tls_state);
    tx(&state.db, move |db_tx| {
        good_ormning::sqlite::good_query!(
            db,
            r#"update identity set tls_state = ${opt refresh_tls_state_t = Some(&tls_state)} where id = ${identity_id_t = &db_identity}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(());
    }).await.context("Error storing identity TLS state")?;
    return Ok(());
}

async fn set_values(
    state: &State,
    identity: Identity,
    tls_state: &stored_self_tls::latest::RefreshTlsState,
) -> Result<(), loga::Error> {
    let publish = &state.spagh_publish;
    publish
        .certs
        .0
        .write()
        .unwrap()
        .insert(identity, load_certified_key(&tls_state.current.pub_pem, &tls_state.current.priv_pem)?);
    let mut certs = vec![tls_state.current.pub_pem.clone()];
    if let Some(pending) = &tls_state.pending {
        certs.push(pending.pub_pem.clone());
    }
    let mut set = HashMap::new();
    set.insert(vec![SPAGH_RECORD_KEY.to_string()], record::RecordValue::latest(record::latest::RecordValue {
        ttl: record::latest::Minutes::from(RECORD_TTL),
        data: Some(publish.record.clone()),
    }));
    set.insert(vec![tls_record::KEY_SUFFIX_TLS.to_string()], record::RecordValue::latest(record::latest::RecordValue {
        ttl: record::latest::Minutes::from(RECORD_TTL),
        data: Some(
            serde_json::to_value(&tls_record::TlsCerts::latest(tls_record::latest::TlsCerts(certs))).unwrap(),
        ),
    }));
    publish.publisher.modify_values(&identity, PublishArgs {
        clear_all: true,
        set: set,
        ..Default::default()
    }).await.context_with("Error setting published values for identity", ea!(identity = identity))?;
    return Ok(());
}

pub async fn publish_identity(state: &State, identity: Identity) -> Result<(), loga::Error> {
    let db_identity = DbIdentity(identity);
    let row = tx(&state.db, move |db_tx| {
        return Ok(good_ormning::sqlite::good_query_opt!(
            db,
            r#"select secret, tls_state from identity where id = ${identity_id_t = &db_identity} and soft_deleted_at is null"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?.context_with("Identity to publish not found", ea!(identity = identity))?);
    }).await?;
    let signer = Arc::new(Mutex::new(local_signer(row.secret.0)?)) as Arc<Mutex<dyn IdentitySigner>>;
    let tls_state = match row.tls_state {
        Some(RefreshTlsState::V1(s)) => s,
        None => {
            let s = stored_self_tls::latest::RefreshTlsState {
                current: request_cert(&state.log, signer.clone(), cert_options()).await?,
                pending: None,
            };
            store_tls_state(state, identity, s.clone()).await?;
            s
        },
    };
    set_values(state, identity, &tls_state).await?;
    let announcement = generate_publish_announce(&signer, vec![InfoResponse {
        advertise_addr: state.spagh_publish.publisher_info.advertise_addr,
        cert_pub_hash: state.spagh_publish.publisher_info.cert_pub_hash.clone(),
    }]).map_err(|e| loga::err_with("Error generating announcement", ea!(identity = identity, err = e)))?;
    state
        .spagh_publish
        .publisher
        .announce(&identity, announcement)
        .await
        .context_with("Error announcing identity", ea!(identity = identity))?;
    return Ok(());
}

pub async fn unpublish_identity(state: &State, identity: Identity) -> Result<(), loga::Error> {
    state.spagh_publish.certs.0.write().unwrap().remove(&identity);
    state
        .spagh_publish
        .publisher
        .clear_identity(&identity)
        .await
        .context_with("Error clearing published identity", ea!(identity = identity))?;
    return Ok(());
}

pub async fn publish_all(state: &State) -> Result<(), loga::Error> {
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
}

pub async fn refresh_certs(state: &State) -> Result<(), loga::Error> {
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
                        Arc::new(Mutex::new(local_signer(row.secret.0)?)) as Arc<Mutex<dyn IdentitySigner>>;
                    tls_state.pending = Some(request_cert(&state.log, signer, cert_options()).await?);
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
}
