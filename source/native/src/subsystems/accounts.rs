use {
    crate::{
        db,
        dbutil::tx,
        interface::{
            config::{
                FdapConfig,
                MaybeFdap,
                UsersConfig,
            },
            db::{
                DbAccountId,
                DbChannelId,
                DbIdentity,
                DbIdentitySecret,
            },
            AccountExternalId,
        },
        subsystems::{
            c2s::{
                insert_channel_row,
                random_channel_id,
            },
            oidc::{
                get_req_session,
                session_lookup,
            },
        },
        State,
    },
    http::{
        HeaderMap,
        Uri,
    },
    loga::{
        ea,
        ResultContext,
    },
    moka::future::Cache,
    shared::interface::shared::AccountId,
    spaghettinuum_native::utils::identity_secret::new_local_identity_secret,
    std::{
        collections::HashSet,
        str::FromStr,
        time::Duration,
    },
};

const DEFAULT_IDENTITY_NAME: &str = "Default";

pub enum UsersState {
    Fdap {
        client: fdap::Client,
        subpath: Vec<String>,
        cache: Cache<AccountExternalId, bool>,
    },
    Local {
        users: HashSet<AccountExternalId>,
    },
}

pub fn build_users_state(
    users: &MaybeFdap<UsersConfig>,
    fdap: &Option<FdapConfig>,
) -> Result<UsersState, loga::Error> {
    match users {
        MaybeFdap::Fdap(subpath) => {
            let Some(fdap) = fdap else {
                return Err(loga::err("User config set to use FDAP but no FDAP configured at config root"));
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
}

fn rowid_to_account(rowid: i64) -> AccountId {
    return AccountId(u64::from_ne_bytes(rowid.to_ne_bytes()));
}

pub async fn identify_c2s(state: &State, headers: &HeaderMap) -> Result<Option<AccountId>, loga::Error> {
    let Some(session) = get_req_session(&state.log, headers) else {
        return Ok(None);
    };
    let Some(user) = session_lookup(&state.db, session).await? else {
        return Ok(None);
    };
    let allowed = match &state.users_state {
        UsersState::Fdap { client, subpath, cache } => cache.try_get_with::<_, loga::Error>(user.clone(), {
            let client = client.clone();
            let subpath = subpath.clone();
            let user = user.clone();
            async move {
                let found =
                    client
                        .user_get(htwrap::htreq::Limits {
                            read_body_size: 1024 * 1024,
                            ..Default::default()
                        }, &user.0, subpath.iter())
                        .await
                        .map_err(|e| loga::err(e.0))
                        .context_with("Error making request to FDAP server", ea!(user = user.0))?;
                let Some(json) = found else {
                    return Ok(false);
                };
                serde_json::from_value::<crate::interface::config::UserConfig>(
                    json,
                ).context_with("User config in FDAP doesn't match expected schema", ea!(user = user.0))?;
                return Ok(true);
            }
        }).await.map_err(|e| e.as_ref().clone())?,
        UsersState::Local { users } => users.contains(&user),
    };
    if !allowed {
        state.log.log_with(loga::DEBUG, "Session user is not an allowed user", ea!(user = user.0));
        return Ok(None);
    }
    let page_size = state.default_page_size as i64;
    let (rowid, new_identity) = tx(&state.db, move |db_tx| {
        let inserted = good_ormning::sqlite::good_query_opt!(
            db,
            r#"insert into account (external_id) values (${string = user.0.as_str()}) on conflict do nothing returning rowid"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        let new_identity = match inserted {
            Some(rowid) => {
                let account_id = DbAccountId(rowid_to_account(rowid));
                let secret = new_local_identity_secret();
                let identity = secret.identity().context("Generated identity secret produced no public key")?;
                let idem = identity.to_string();
                let db_identity = DbIdentity(identity);
                let db_secret = DbIdentitySecret(secret);
                good_ormning::sqlite::good_query!(
                    db,
                    r#"insert into identity (account_id, id, idem, name, description, secret) values (${account_id_t = &account_id}, ${identity_id_t = &db_identity}, ${string = idem.as_str()}, ${string = DEFAULT_IDENTITY_NAME}, ${string = ""}, ${identity_secret_t = &db_secret})"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
                insert_channel_row(
                    db_tx,
                    account_id,
                    db_identity,
                    &DbChannelId(random_channel_id()),
                    db_identity,
                    "",
                    None,
                    "notes",
                    "",
                    None,
                    None,
                    None,
                    page_size,
                    true,
                )?;
                Some(identity)
            },
            None => None,
        };
        let rowid = good_ormning::sqlite::good_query_opt!(
            db,
            r#"select rowid from account where external_id = ${string = user.0.as_str()} and soft_deleted_at is null"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok((rowid, new_identity));
    }).await.context("Error ensuring account row")?;
    if let Some(identity) = new_identity {
        crate::subsystems::spagh_publish::publish_identity(state, identity).await?;
    }
    return Ok(rowid.map(rowid_to_account));
}
