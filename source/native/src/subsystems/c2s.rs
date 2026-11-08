use {
    crate::{
        db,
        interface::db::{
            DbAccountId,
            DbChannelGroupId,
            DbChannelId,
            DbIdentity,
        },
        subsystems::{
            files::{
                file_ref,
                store_file,
            },
            messages::{
                retention_from_db,
                RETENTION_MODE_ALL,
            },
        },
        State,
    },
    loga::{
        ea,
        ResultContext,
    },
    rand::Rng,
    rusqlite::Transaction,
    shared::interface::{
        shared::{
            ChannelGroupId,
            ChannelId,
            FileHash,
            FileRef,
            JoinPortrait,
            Portrait,
            QualifiedChannelId,
        },
        wire::c2s::{
            ChannelRetention,
            ChannelRes,
            ModifyOption,
        },
        wire::kwaurl::KwaUrl,
    },
    spaghettinuum::interface::identity::Identity,
};

pub const PORTRAIT_MAX_BYTES: usize = 1024 * 1024;

pub fn validate_portrait(mime_type: &str, data: &[u8]) -> Result<(), loga::Error> {
    if !{
        let value: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif", "image/avif"];
        value
    }.contains(&mime_type) {
        return Err(loga::err_with("Unsupported portrait mime type", ea!(mime_type = mime_type)));
    }
    if data.len() > PORTRAIT_MAX_BYTES {
        return Err(loga::err_with("Portrait is too large", ea!(size = data.len(), max = PORTRAIT_MAX_BYTES)));
    }
    return Ok(());
}

pub fn random_token() -> String {
    let mut buf = [0u8; 32];
    rand::rng().fill_bytes(&mut buf);
    return hex::encode(&buf);
}

pub fn random_channel_id() -> ChannelId {
    let mut buf = [0u8; 8];
    rand::rng().fill_bytes(&mut buf);
    return ChannelId(u64::from_ne_bytes(buf));
}

pub fn opt_idem(idem: String) -> Option<String> {
    if idem.is_empty() {
        return None;
    } else {
        return Some(idem);
    }
}

pub fn require_owned_identity(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    identity: DbIdentity,
) -> Result<(), loga::Error> {
    let found = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select 1 as x from identity where account_id = ${account_id_t = &account} and id = ${identity_id_t = &identity} and soft_deleted_at is null"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    if found.is_none() {
        return Err(loga::err_with("Identity doesn't belong to this account", ea!(identity = identity.0)));
    }
    return Ok(());
}

pub fn require_owned_channel(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
) -> Result<(), loga::Error> {
    let local =
        crate::subsystems::messages::local_channel_get(db_tx, account, owner, channel)?.context("No such channel")?;
    if local.local_identity != owner.0 {
        return Err(loga::err("Channel isn't owned by this account"));
    }
    return Ok(());
}

pub fn validate_join_name(name: &str) -> Result<(), loga::Error> {
    if name.is_empty() {
        return Err(loga::err("A name is required when joining"));
    }
    return Ok(());
}

pub enum PortraitChange {
    Keep,
    Clear,
    Set(FileHash),
}

impl PortraitChange {
    pub fn apply(&self, current: Option<String>) -> Option<String> {
        match self {
            PortraitChange::Keep => return current,
            PortraitChange::Clear => return None,
            PortraitChange::Set(h) => return Some(h.0.clone()),
        }
    }
}

pub async fn portrait_store(state: &State, account: DbAccountId, p: &Portrait) -> Result<FileHash, loga::Error> {
    validate_portrait(&p.mime_type, &p.data.0)?;
    return Ok(store_file(state, account, p.mime_type.clone(), p.data.0.clone()).await?.hash);
}

pub async fn resolve_modify_portrait(
    state: &State,
    account: DbAccountId,
    m: &Option<ModifyOption<Portrait>>,
) -> Result<PortraitChange, loga::Error> {
    match m {
        None => return Ok(PortraitChange::Keep),
        Some(ModifyOption { value: None }) => return Ok(PortraitChange::Clear),
        Some(ModifyOption { value: Some(p) }) => return Ok(
            PortraitChange::Set(portrait_store(state, account, p).await?),
        ),
    }
}

pub fn join_remote_from_url(
    url: &KwaUrl,
) -> Result<(Identity, crate::interface::s2s::JoinPreviewTarget, crate::interface::s2s::FileAuth), loga::Error> {
    use crate::interface::s2s::{
        FileAuth,
        JoinPreviewTarget,
    };

    match url {
        KwaUrl::IdentityInvite(u) => return Ok((u.identity, JoinPreviewTarget::IdentityInvite {
            identity: u.identity,
            code: u.code.clone(),
        }, FileAuth::IdentityInvite {
            identity: u.identity,
            code: u.code.clone(),
        })),
        KwaUrl::ChannelInvite(u) => return Ok((u.channel.identity, JoinPreviewTarget::ChannelInvite {
            channel: u.channel.clone(),
            code: u.code.clone(),
        }, FileAuth::ChannelInvite {
            channel: u.channel.clone(),
            code: u.code.clone(),
        })),
        KwaUrl::Channel(c) => return Ok((c.identity, JoinPreviewTarget::Channel(c.clone()), FileAuth::Caller)),
        KwaUrl::Message(_) => return Err(loga::err("A message URL can't be joined")),
    }
}

pub async fn fetch_join_portrait(
    state: &State,
    url: &KwaUrl,
    hash: FileHash,
) -> Result<(String, Vec<u8>), loga::Error> {
    let (host, _, auth) = join_remote_from_url(url)?;
    let (mime, data) = {
        use {
            crate::interface::s2s::GetFile,
            http::Method,
            shared::interface::wire::c2s::PathReqTrait,
            spaghettinuum::interface::identity::Identity,
            std::collections::HashMap,
        };
        use crate::subsystems::s2s_client::*;

        let ret: Result<(String, Vec<u8>), loga::Error> = async {
            let (s2s_client, target, signer, req): (&S2sClient, Identity, Option<&Signer>, GetFile) =
                (&(state.s2s_client), host, None, GetFile {
                    auth: auth,
                    hash: hash,
                });
            let path = format!("/{}", req.serialize_path());
            let Some(signer) = signer else {
                let (mime, body, _) =
                    s2s_client.request_raw(target, Method::GET, &path, &HashMap::new(), vec![]).await?;
                return Ok((mime, body));
            };
            let identity = signer_identity(signer)?;
            let token = s2s_client.authenticate(target, signer).await?;
            let res = s2s_client.request_raw(target, Method::GET, &path, &auth_headers(&token), vec![]).await;
            if res.is_err() {
                s2s_client.tokens.invalidate(&(target, identity)).await;
            }
            let (mime, body, _) = res?;
            return Ok((mime, body));
        }.await;
        ret
    }.context("s2s file fetch failed")?;
    validate_portrait(&mime, &data).context("Remote portrait rejected")?;
    return Ok((mime, data));
}

pub async fn resolve_join_portrait(
    state: &State,
    account: DbAccountId,
    url: &KwaUrl,
    portrait: Option<JoinPortrait>,
) -> Result<Option<FileHash>, loga::Error> {
    use crate::interface::s2s::GetJoinPreview;

    match portrait {
        None => return Ok(None),
        Some(JoinPortrait::Upload(p)) => return Ok(Some(portrait_store(state, account, &p).await?)),
        Some(JoinPortrait::FromInvitation) => {
            let (host, target, _) = join_remote_from_url(url)?;
            let preview =
                state
                    .s2s_client
                    .get_join_preview(host, GetJoinPreview { target: target })
                    .await
                    .context("s2s join preview failed")?;
            let Some(remote) = preview.portrait else {
                return Err(loga::err("Invitation has no portrait to copy"));
            };
            let (mime, data) = fetch_join_portrait(state, url, remote.hash).await?;
            return Ok(Some(store_file(state, account, mime, data).await?.hash));
        },
    }
}

pub struct ChannelRow {
    pub owner: Identity,
    pub id: ChannelId,
    pub local_identity: Identity,
    pub idem: String,
    pub group: Option<ChannelGroupId>,
    pub name: String,
    pub description: String,
    pub shown_name: Option<String>,
    pub shown_description: Option<String>,
    pub portrait: Option<FileRef>,
    pub retention: ChannelRetention,
    pub public_read: bool,
    pub public_post: bool,
}

impl ChannelRow {
    pub fn into_res(self) -> ChannelRes {
        return ChannelRes {
            own_identity: self.local_identity,
            id: QualifiedChannelId {
                identity: self.owner,
                channel: self.id,
            },
            idem: opt_idem(self.idem),
            group: self.group,
            name: self.name,
            description: self.description,
            shown_name: self.shown_name,
            shown_description: self.shown_description,
            portrait: self.portrait,
            retention: self.retention,
            public_read: self.public_read,
            public_post: self.public_post,
        };
    }
}

pub fn insert_channel_row(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    local_identity: DbIdentity,
    idem: &str,
    group: Option<DbChannelGroupId>,
    name: &str,
    description: &str,
    shown_name: Option<&str>,
    shown_description: Option<&str>,
    portrait: Option<&str>,
    page_size: i64,
    authenticated: bool,
) -> Result<(), loga::Error> {
    let is_owner = local_identity.0 == owner.0;
    good_ormning::sqlite::good_query!(
        db,
        r#"insert into channel (account_id, identity, id, local_identity, idem, channel_group, name, description, portrait, page_size, authenticated, public_read, public_post, retention_mode, backfill_done, next_sender_unique) values (${account_id_t = &account}, ${identity_id_t = &owner}, ${channel_id_t = channel}, ${identity_id_t = &local_identity}, ${string = idem}, ${opt channelgroup_id_t = group.as_ref()}, ${string = name}, ${string = description}, ${opt string = portrait}, ${i64 = page_size}, ${bool = authenticated}, ${bool = false}, ${bool = false}, ${string = RETENTION_MODE_ALL}, ${bool = is_owner}, ${i64 = 0})"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    if shown_name.is_some() || shown_description.is_some() {
        good_ormning::sqlite::good_query!(
            db,
            r#"update channel set shown_name = ${opt string = shown_name}, shown_description = ${opt string = shown_description} where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and id = ${channel_id_t = channel}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
    }
    return Ok(());
}

pub fn channel_row_get(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
) -> Result<Option<ChannelRow>, loga::Error> {
    let row = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select identity, id, local_identity, idem, channel_group, name, description, shown_name, shown_description, portrait, retention_secs, retention_mode, public_read, public_post from channel where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and id = ${channel_id_t = channel} and deleted is null"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    let Some(r) = row else {
        return Ok(None);
    };
    return Ok(Some(ChannelRow {
        owner: r.identity.0,
        id: r.id.0,
        local_identity: r.local_identity.0,
        idem: r.idem,
        group: r.channel_group.map(|g| g.0),
        name: r.name,
        description: r.description,
        shown_name: r.shown_name,
        shown_description: r.shown_description,
        portrait: file_ref(db_tx, account, r.portrait)?,
        retention: retention_from_db(&r.retention_mode, r.retention_secs),
        public_read: r.public_read,
        public_post: r.public_post,
    }));
}

pub struct PushResult {
    pub seq: Option<u64>,
    pub recipients: Vec<Identity>,
}

pub fn insert_joined_channel(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    local_identity: DbIdentity,
    idem: &str,
    name: &str,
    description: &str,
    portrait: Option<&str>,
    page_size: i64,
    authenticated: bool,
) -> Result<(), loga::Error> {
    let existing = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select local_identity, authenticated from channel where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and id = ${channel_id_t = channel}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    if let Some(existing) = existing {
        if existing.local_identity.0 != local_identity.0 {
            return Err(
                loga::err_with(
                    "This account already participates in the channel with a different identity",
                    ea!(identity = existing.local_identity.0),
                ),
            );
        }
        let authenticated = existing.authenticated || authenticated;
        good_ormning::sqlite::good_query!(
            db,
            r#"update channel set deleted = ${opt time_ms_t = None}, authenticated = ${bool = authenticated}, name = ${string = name}, description = ${string = description}, portrait = ${opt string = portrait} where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and id = ${channel_id_t = channel}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(());
    }
    insert_channel_row(
        db_tx,
        account,
        owner,
        channel,
        local_identity,
        idem,
        None,
        name,
        description,
        None,
        None,
        portrait,
        page_size,
        authenticated,
    )?;
    return Ok(());
}
