use {
    crate::{
        db,
        dbutil::tx,
        interface::db::{
            DbAccountId,
            DbChannelGroupId,
            DbChannelId,
            DbIdentity,
            DbIdentitySecret,
        },
        subsystems::{
            files::{
                file_ref,
                store_file,
            },
            messages::{
                channel_members,
                i64_to_u64,
                ingest_message,
                load_signer_tx,
                notify_remotes,
                publish_local_notification,
            },
            pull_scheduler::PullKey,
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
            AccountId,
            ChannelGroupId,
            ChannelId,
            ChannelInviteId,
            ChannelInviteToken,
            IdentityInviteId,
            IdentityInviteToken,
            Message,
            MessageBody,
            MessageId,
            MessageRel,
            FileHash,
            FileRef,
            JoinPortrait,
            Portrait,
            QualifiedChannelId,
            QualifiedChannelInviteToken,
            QualifiedIdentityInviteToken,
        },
        wire::c2s::{
            ChannelAdd,
            ChannelJoinPreview,
            ChannelJoinPreviewRes,
            ChannelCreate,
            ChannelDelete,
            ChannelGroupCreate,
            ChannelGroupDelete,
            ChannelGroupModify,
            ChannelGroupRes,
            ChannelInviteCreate,
            ChannelInviteDelete,
            ChannelInviteList,
            ChannelInviteModify,
            ChannelInviteRes,
            ChannelJoinChannel,
            ChannelJoinIdentity,
            ChannelList,
            ChannelMemberDelete,
            ChannelMemberList,
            ChannelMemberModify,
            ChannelMemberRes,
            ChannelModify,
            ChannelRes,
            ContactList,
            ContactModify,
            ModifyOption,
            ContactRes,
            IdentityCreate,
            IdentityDelete,
            IdentityInviteCreate,
            IdentityInviteDelete,
            IdentityInviteModify,
            IdentityInviteRes,
            IdentityModify,
            IdentityRes,
            MessagePush,
            NotificationRegister,
        },
        wire::kwaurl::{
            KwaUrl,
            KwaUrlChannelInvite,
            KwaUrlIdentityInvite,
        },
    },
    spaghettinuum::{
        byteszb32::BytesZb32,
        interface::identity::Identity,
        jsonsig::JsonSignature,
        sigdomain::SigDomained,
    },
    spaghettinuum_native::utils::identity_secret::{
        new_local_identity_secret,
        IdentitySigner,
    },
    std::sync::Arc,
};

pub const PORTRAIT_MAX_BYTES: usize = 1024 * 1024;
pub const PORTRAIT_MIME_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp", "image/gif", "image/avif"];

pub fn validate_portrait(mime_type: &str, data: &[u8]) -> Result<(), loga::Error> {
    if !PORTRAIT_MIME_TYPES.contains(&mime_type) {
        return Err(loga::err_with("Unsupported portrait mime type", ea!(mime_type = mime_type)));
    }
    if data.len() > PORTRAIT_MAX_BYTES {
        return Err(loga::err_with("Portrait is too large", ea!(size = data.len(), max = PORTRAIT_MAX_BYTES)));
    }
    return Ok(());
}

fn random_token() -> String {
    let mut buf = [0u8; 32];
    rand::rng().fill_bytes(&mut buf);
    return hex::encode(&buf);
}

pub fn random_channel_id() -> ChannelId {
    let mut buf = [0u8; 8];
    rand::rng().fill_bytes(&mut buf);
    return ChannelId(u64::from_ne_bytes(buf));
}

fn opt_idem(idem: String) -> Option<String> {
    if idem.is_empty() {
        return None;
    } else {
        return Some(idem);
    }
}

fn require_owned_identity(
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

fn require_owned_channel(
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

pub async fn identity_create(
    state: &State,
    account: AccountId,
    req: IdentityCreate,
) -> Result<IdentityRes, loga::Error> {
    let account_id = DbAccountId(account);
    let secret = new_local_identity_secret();
    let identity = secret.identity().context("Generated identity secret produced no public key")?;
    let idem = req.idem.clone().unwrap_or_else(|| identity.to_string());
    let row = tx(&state.db, move |db_tx| {
        let existing = good_ormning::sqlite::good_query_opt!(
            db,
            r#"select id, idem, name, description, shown_name, shown_description, portrait from identity where account_id = ${account_id_t = &account_id} and idem = ${string = idem.as_str()} and soft_deleted_at is null"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        if let Some(existing) = existing {
            return Ok(IdentityRes {
                id: existing.id.0,
                idem: opt_idem(existing.idem),
                name: existing.name,
                description: existing.description,
                shown_name: existing.shown_name,
                shown_description: existing.shown_description,
                portrait: file_ref(db_tx, account_id, existing.portrait)?,
            });
        }
        let db_identity = DbIdentity(identity);
        let db_secret = DbIdentitySecret(secret);
        good_ormning::sqlite::good_query!(
            db,
            r#"insert into identity (account_id, id, idem, name, description, shown_name, shown_description, secret) values (${account_id_t = &account_id}, ${identity_id_t = &db_identity}, ${string = idem.as_str()}, ${string = req.name.as_str()}, ${string = req.description.as_str()}, ${opt string = req.shown_name.as_deref()}, ${opt string = req.shown_description.as_deref()}, ${identity_secret_t = &db_secret})"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(IdentityRes {
            id: identity,
            idem: opt_idem(idem),
            name: req.name,
            description: req.description,
            shown_name: req.shown_name,
            shown_description: req.shown_description,
            portrait: None,
        });
    }).await.context("Error inserting identity")?;
    crate::subsystems::spagh_publish::publish_identity(state, row.id).await?;
    return Ok(row);
}

pub async fn identity_list(state: &State, account: AccountId) -> Result<Vec<IdentityRes>, loga::Error> {
    let account_id = DbAccountId(account);
    return Ok(tx(&state.db, move |db_tx| {
        let rows = good_ormning::sqlite::good_query_many!(
            db,
            r#"select id, idem, name, description, shown_name, shown_description, portrait from identity where account_id = ${account_id_t = &account_id} and soft_deleted_at is null order by id"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        let mut out = vec![];
        for r in rows {
            out.push(IdentityRes {
                id: r.id.0,
                idem: opt_idem(r.idem),
                name: r.name,
                description: r.description,
                shown_name: r.shown_name,
                shown_description: r.shown_description,
                portrait: file_ref(db_tx, account_id, r.portrait)?,
            });
        }
        return Ok(out);
    }).await.context("Error listing identities")?);
}

pub async fn identity_modify(
    state: &State,
    account: AccountId,
    req: IdentityModify,
) -> Result<IdentityRes, loga::Error> {
    let account_id = DbAccountId(account);
    let db_identity = DbIdentity(req.id);
    let portrait = resolve_modify_portrait(state, account_id, &req.portrait).await?;
    let row = tx(&state.db, move |db_tx| {
        require_owned_identity(db_tx, account_id, db_identity)?;
        if let Some(v) = &req.name {
            good_ormning::sqlite::good_query!(
                db,
                r#"update identity set name = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(v) = &req.description {
            good_ormning::sqlite::good_query!(
                db,
                r#"update identity set description = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(m) = &req.shown_name {
            good_ormning::sqlite::good_query!(
                db,
                r#"update identity set shown_name = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(m) = &req.shown_description {
            good_ormning::sqlite::good_query!(
                db,
                r#"update identity set shown_description = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if !matches!(portrait, PortraitChange::Keep) {
            let current = good_ormning::sqlite::good_query_opt!(
                db,
                r#"select portrait from identity where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?.flatten();
            let next = portrait.apply(current);
            good_ormning::sqlite::good_query!(
                db,
                r#"update identity set portrait = ${opt string = next.as_deref()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        let row = good_ormning::sqlite::good_query_opt!(
            db,
            r#"select id, idem, name, description, shown_name, shown_description, portrait from identity where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity} and soft_deleted_at is null"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?.context("No such identity")?;
        return Ok(IdentityRes {
            id: row.id.0,
            idem: opt_idem(row.idem),
            name: row.name,
            description: row.description,
            shown_name: row.shown_name,
            shown_description: row.shown_description,
            portrait: file_ref(db_tx, account_id, row.portrait)?,
        });
    }).await.context("Error modifying identity")?;
    return Ok(row);
}

fn validate_join_name(name: &str) -> Result<(), loga::Error> {
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
    fn apply(&self, current: Option<String>) -> Option<String> {
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

async fn resolve_modify_portrait(
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
    use crate::interface::s2s::GetFile;

    let (host, _, auth) = join_remote_from_url(url)?;
    let (mime, data) = state.s2s_client.get_file(host, None, GetFile {
        auth: auth,
        hash: hash,
    }).await.context("s2s file fetch failed")?;
    validate_portrait(&mime, &data).context("Remote portrait rejected")?;
    return Ok((mime, data));
}

async fn resolve_join_portrait(
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

pub async fn identity_delete(state: &State, account: AccountId, req: IdentityDelete) -> Result<(), loga::Error> {
    let account_id = DbAccountId(account);
    let db_identity = DbIdentity(req.id);
    let now = jiff::Timestamp::now();
    tx(&state.db, move |db_tx| {
        good_ormning::sqlite::good_query!(
            db,
            r#"update identity set soft_deleted_at = ${time_ms_t = &crate::interface::db::DbTimeMs(now)} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_identity}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(());
    }).await.context("Error deleting identity")?;
    crate::subsystems::spagh_publish::unpublish_identity(state, req.id).await?;
    return Ok(());
}

pub async fn channel_group_create(
    state: &State,
    account: AccountId,
    req: ChannelGroupCreate,
) -> Result<ChannelGroupRes, loga::Error> {
    let account_id = DbAccountId(account);
    let idem = req.idem.clone().unwrap_or_default();
    let row = tx(&state.db, move |db_tx| {
        if !idem.is_empty() {
            let existing = good_ormning::sqlite::good_query_opt!(
                db,
                r#"select rowid, idem, name, description from channelgroup where account_id = ${account_id_t = &account_id} and idem = ${string = idem.as_str()}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
            if let Some(existing) = existing {
                return Ok((existing.rowid, existing.idem, existing.name, existing.description));
            }
        }
        good_ormning::sqlite::good_query!(
            db,
            r#"insert into channelgroup (account_id, idem, name, description) values (${account_id_t = &account_id}, ${string = idem.as_str()}, ${string = req.name.as_str()}, ${string = req.description.as_str()})"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok((db_tx.0.last_insert_rowid(), idem, req.name, req.description));
    }).await.context("Error inserting channel group")?;
    return Ok(ChannelGroupRes {
        id: ChannelGroupId(i64_to_u64(row.0)),
        idem: opt_idem(row.1),
        name: row.2,
        description: row.3,
    });
}

pub async fn channel_group_list(state: &State, account: AccountId) -> Result<Vec<ChannelGroupRes>, loga::Error> {
    let account_id = DbAccountId(account);
    let rows = tx(&state.db, move |db_tx| {
        return Ok(good_ormning::sqlite::good_query_many!(
            db,
            r#"select rowid, idem, name, description from channelgroup where account_id = ${account_id_t = &account_id} order by rowid"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?);
    }).await.context("Error listing channel groups")?;
    return Ok(rows.into_iter().map(|r| ChannelGroupRes {
        id: ChannelGroupId(i64_to_u64(r.rowid)),
        idem: opt_idem(r.idem),
        name: r.name,
        description: r.description,
    }).collect());
}

pub async fn channel_group_modify(
    state: &State,
    account: AccountId,
    req: ChannelGroupModify,
) -> Result<ChannelGroupRes, loga::Error> {
    let account_id = DbAccountId(account);
    let gid = DbChannelGroupId(req.id);
    let row = tx(&state.db, move |db_tx| {
        if let Some(v) = &req.name {
            good_ormning::sqlite::good_query!(
                db,
                r#"update channelgroup set name = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and rowid = ${channelgroup_id_t = &gid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(v) = &req.description {
            good_ormning::sqlite::good_query!(
                db,
                r#"update channelgroup set description = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and rowid = ${channelgroup_id_t = &gid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        return Ok(good_ormning::sqlite::good_query_opt!(
            db,
            r#"select rowid, idem, name, description from channelgroup where account_id = ${account_id_t = &account_id} and rowid = ${channelgroup_id_t = &gid}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?.context("No such channel group")?);
    }).await.context("Error modifying channel group")?;
    return Ok(ChannelGroupRes {
        id: ChannelGroupId(i64_to_u64(row.rowid)),
        idem: opt_idem(row.idem),
        name: row.name,
        description: row.description,
    });
}

pub async fn channel_group_delete(
    state: &State,
    account: AccountId,
    req: ChannelGroupDelete,
) -> Result<(), loga::Error> {
    let account_id = DbAccountId(account);
    let gid = DbChannelGroupId(req.id);
    tx(&state.db, move |db_tx| {
        good_ormning::sqlite::good_query!(
            db,
            r#"update channel set channel_group = ${opt channelgroup_id_t = None} where account_id = ${account_id_t = &account_id} and channel_group = ${channelgroup_id_t = &gid}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        good_ormning::sqlite::good_query!(
            db,
            r#"delete from channelgroup where account_id = ${account_id_t = &account_id} and rowid = ${channelgroup_id_t = &gid}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(());
    }).await.context("Error deleting channel group")?;
    return Ok(());
}

pub async fn identity_invite_create(
    state: &State,
    account: AccountId,
    req: IdentityInviteCreate,
) -> Result<IdentityInviteRes, loga::Error> {
    let account_id = DbAccountId(account);
    let identity = req.identity;
    let db_identity = DbIdentity(identity);
    let idem = req.idem.clone().unwrap_or_default();
    let token = random_token();
    return Ok(tx(&state.db, move |db_tx| {
        require_owned_identity(db_tx, account_id, db_identity)?;
        if !idem.is_empty() {
            let existing = good_ormning::sqlite::good_query_opt!(
                db,
                r#"select rowid, token, name, description, shown_name, shown_description, single_use, expiry from identity_invite where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_identity} and idem = ${string = idem.as_str()}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
            if let Some(e) = existing {
                return Ok(IdentityInviteRes {
                    id: IdentityInviteId(i64_to_u64(e.rowid)),
                    token: QualifiedIdentityInviteToken {
                        identity: identity,
                        token: IdentityInviteToken(e.token),
                    },
                    name: e.name,
                    description: e.description,
                    shown_name: e.shown_name,
                    shown_description: e.shown_description,
                    single_use: e.single_use,
                    expiry: e.expiry.map(|t| t.0),
                });
            }
        }
        good_ormning::sqlite::good_query!(
            db,
            r#"insert into identity_invite (account_id, identity, idem, token, name, description, shown_name, shown_description, single_use, expiry, used_count) values (${account_id_t = &account_id}, ${identity_id_t = &db_identity}, ${string = idem.as_str()}, ${string = token.as_str()}, ${string = req.name.as_str()}, ${string = req.description.as_str()}, ${opt string = req.shown_name.as_deref()}, ${opt string = req.shown_description.as_deref()}, ${bool = req.single_use}, ${opt time_ms_t = req.expiry.map(crate::interface::db::DbTimeMs).as_ref()}, ${i64 = 0})"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(IdentityInviteRes {
            id: IdentityInviteId(i64_to_u64(db_tx.0.last_insert_rowid())),
            token: QualifiedIdentityInviteToken {
                identity: identity,
                token: IdentityInviteToken(token),
            },
            name: req.name,
            description: req.description,
            shown_name: req.shown_name,
            shown_description: req.shown_description,
            single_use: req.single_use,
            expiry: req.expiry,
        });
    }).await.context("Error inserting identity invite")?);
}

pub async fn identity_invite_list(state: &State, account: AccountId) -> Result<Vec<IdentityInviteRes>, loga::Error> {
    let account_id = DbAccountId(account);
    let rows = tx(&state.db, move |db_tx| {
        return Ok(good_ormning::sqlite::good_query_many!(
            db,
            r#"select rowid, identity, token, name, description, shown_name, shown_description, single_use, expiry from identity_invite where account_id = ${account_id_t = &account_id} order by rowid"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?);
    }).await.context("Error listing identity invites")?;
    return Ok(rows.into_iter().map(|r| IdentityInviteRes {
        id: IdentityInviteId(i64_to_u64(r.rowid)),
        token: QualifiedIdentityInviteToken {
            identity: r.identity.0,
            token: IdentityInviteToken(r.token),
        },
        name: r.name,
        description: r.description,
        shown_name: r.shown_name,
        shown_description: r.shown_description,
        single_use: r.single_use,
        expiry: r.expiry.map(|t| t.0),
    }).collect());
}

pub async fn identity_invite_modify(
    state: &State,
    account: AccountId,
    req: IdentityInviteModify,
) -> Result<IdentityInviteRes, loga::Error> {
    let account_id = DbAccountId(account);
    let rowid = crate::subsystems::messages::u64_to_i64(req.id.0);
    let row = tx(&state.db, move |db_tx| {
        if let Some(v) = &req.name {
            good_ormning::sqlite::good_query!(
                db,
                r#"update identity_invite set name = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(v) = &req.description {
            good_ormning::sqlite::good_query!(
                db,
                r#"update identity_invite set description = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(m) = &req.shown_name {
            good_ormning::sqlite::good_query!(
                db,
                r#"update identity_invite set shown_name = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(m) = &req.shown_description {
            good_ormning::sqlite::good_query!(
                db,
                r#"update identity_invite set shown_description = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(v) = req.single_use {
            good_ormning::sqlite::good_query!(
                db,
                r#"update identity_invite set single_use = ${bool = v} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(m) = req.expiry {
            let v = m.value;
            good_ormning::sqlite::good_query!(
                db,
                r#"update identity_invite set expiry = ${opt time_ms_t = v.map(crate::interface::db::DbTimeMs).as_ref()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        return Ok(good_ormning::sqlite::good_query_opt!(
            db,
            r#"select rowid, identity, token, name, description, shown_name, shown_description, single_use, expiry from identity_invite where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?.context("No such identity invite")?);
    }).await.context("Error modifying identity invite")?;
    return Ok(IdentityInviteRes {
        id: IdentityInviteId(i64_to_u64(row.rowid)),
        token: QualifiedIdentityInviteToken {
            identity: row.identity.0,
            token: IdentityInviteToken(row.token),
        },
        name: row.name,
        description: row.description,
        shown_name: row.shown_name,
        shown_description: row.shown_description,
        single_use: row.single_use,
        expiry: row.expiry.map(|t| t.0),
    });
}

pub async fn identity_invite_delete(
    state: &State,
    account: AccountId,
    req: IdentityInviteDelete,
) -> Result<(), loga::Error> {
    let account_id = DbAccountId(account);
    let rowid = crate::subsystems::messages::u64_to_i64(req.id.0);
    tx(&state.db, move |db_tx| {
        good_ormning::sqlite::good_query!(
            db,
            r#"delete from identity_invite where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(());
    }).await.context("Error deleting identity invite")?;
    return Ok(());
}

struct ChannelRow {
    owner: Identity,
    id: ChannelId,
    local_identity: Identity,
    idem: String,
    group: Option<ChannelGroupId>,
    name: String,
    description: String,
    shown_name: Option<String>,
    shown_description: Option<String>,
    portrait: Option<FileRef>,
    retention_secs: Option<i64>,
    public_read: bool,
    public_post: bool,
}

const SECS_PER_DAY: i64 = 24 * 60 * 60;

impl ChannelRow {
    fn into_res(self) -> ChannelRes {
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
            retention_days: self.retention_secs.map(|s| (s / SECS_PER_DAY) as u64),
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
        r#"insert into channel (account_id, identity, id, local_identity, idem, channel_group, name, description, portrait, page_size, authenticated, public_read, public_post, retention_secs, backfill_done, next_sender_unique) values (${account_id_t = &account}, ${identity_id_t = &owner}, ${channel_id_t = channel}, ${identity_id_t = &local_identity}, ${string = idem}, ${opt channelgroup_id_t = group.as_ref()}, ${string = name}, ${string = description}, ${opt string = portrait}, ${i64 = page_size}, ${bool = authenticated}, ${bool = false}, ${bool = false}, ${opt i64 = None}, ${bool = is_owner}, ${i64 = 0})"#;
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

fn channel_row_get(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
) -> Result<Option<ChannelRow>, loga::Error> {
    let row = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select identity, id, local_identity, idem, channel_group, name, description, shown_name, shown_description, portrait, retention_secs, public_read, public_post from channel where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and id = ${channel_id_t = channel} and deleted is null"#;
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
        retention_secs: r.retention_secs,
        public_read: r.public_read,
        public_post: r.public_post,
    }));
}

pub async fn channel_create(
    state: &State,
    account: AccountId,
    req: ChannelCreate,
) -> Result<ChannelRes, loga::Error> {
    let account_id = DbAccountId(account);
    let db_identity = DbIdentity(req.identity);
    let db_channel_id = DbChannelId(random_channel_id());
    let idem = req.idem.clone().unwrap_or_default();
    let group = req.group.map(DbChannelGroupId);
    let page_size = state.default_page_size as i64;
    let row = tx(&state.db, move |db_tx| {
        require_owned_identity(db_tx, account_id, db_identity)?;
        if !idem.is_empty() {
            let existing = good_ormning::sqlite::good_query_opt!(
                db,
                r#"select id from channel where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_identity} and idem = ${string = idem.as_str()} and deleted is null"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
            if let Some(existing) = existing {
                return Ok(
                    channel_row_get(
                        db_tx,
                        account_id,
                        db_identity,
                        &existing,
                    )?.context("Channel vanished during lookup")?,
                );
            }
        }
        insert_channel_row(
            db_tx,
            account_id,
            db_identity,
            &db_channel_id,
            db_identity,
            &idem,
            group,
            &req.name,
            &req.description,
            req.shown_name.as_deref(),
            req.shown_description.as_deref(),
            None,
            page_size,
            true,
        )?;
        return Ok(
            channel_row_get(
                db_tx,
                account_id,
                db_identity,
                &db_channel_id,
            )?.context("Channel vanished after insert")?,
        );
    }).await.context("Error inserting channel")?;
    return Ok(row.into_res());
}

pub async fn channel_list(
    state: &State,
    account: AccountId,
    _req: ChannelList,
) -> Result<Vec<ChannelRes>, loga::Error> {
    let account_id = DbAccountId(account);
    let rows = tx(&state.db, move |db_tx| {
        let rows = good_ormning::sqlite::good_query_many!(
            db,
            r#"select identity, id, local_identity, idem, channel_group, name, description, shown_name, shown_description, portrait, retention_secs, public_read, public_post from channel where account_id = ${account_id_t = &account_id} and deleted is null order by identity, id"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        let mut out = vec![];
        for r in rows {
            out.push(ChannelRow {
                owner: r.identity.0,
                id: r.id.0,
                local_identity: r.local_identity.0,
                idem: r.idem,
                group: r.channel_group.map(|g| g.0),
                name: r.name,
                description: r.description,
                shown_name: r.shown_name,
                shown_description: r.shown_description,
                portrait: file_ref(db_tx, account_id, r.portrait)?,
                retention_secs: r.retention_secs,
                public_read: r.public_read,
                public_post: r.public_post,
            });
        }
        return Ok(out);
    }).await.context("Error listing channels")?;
    return Ok(rows.into_iter().map(|r| r.into_res()).collect());
}

pub async fn channel_modify(
    state: &State,
    account: AccountId,
    req: ChannelModify,
) -> Result<ChannelRes, loga::Error> {
    let account_id = DbAccountId(account);
    let db_owner = DbIdentity(req.id.identity);
    let db_channel_id = DbChannelId(req.id.channel.clone());
    let portrait = resolve_modify_portrait(state, account_id, &req.portrait).await?;
    let row = tx(&state.db, move |db_tx| {
        if !matches!(portrait, PortraitChange::Keep) {
            let current = good_ormning::sqlite::good_query_opt!(
                db,
                r#"select portrait from channel where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?.flatten();
            let next = portrait.apply(current);
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel set portrait = ${opt string = next.as_deref()} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(v) = &req.name {
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel set name = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(v) = &req.description {
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel set description = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(m) = &req.shown_name {
            require_owned_channel(db_tx, account_id, db_owner, &db_channel_id)?;
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel set shown_name = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(m) = &req.shown_description {
            require_owned_channel(db_tx, account_id, db_owner, &db_channel_id)?;
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel set shown_description = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(m) = &req.group {
            let g = m.value.clone().map(DbChannelGroupId);
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel set channel_group = ${opt channelgroup_id_t = g.as_ref()} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(m) = &req.retention_days {
            let secs = match m.value {
                Some(days) => Some(
                    i64::try_from(days)
                        .ok()
                        .and_then(|d| d.checked_mul(SECS_PER_DAY))
                        .context("Retention days out of range")?,
                ),
                None => None,
            };
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel set retention_secs = ${opt i64 = secs}, backfill_done = ${bool = false} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id} and local_identity != ${identity_id_t = &db_owner}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel set retention_secs = ${opt i64 = secs} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id} and local_identity = ${identity_id_t = &db_owner}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(v) = req.public_read {
            require_owned_channel(db_tx, account_id, db_owner, &db_channel_id)?;
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel set public_read = ${bool = v} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(v) = req.public_post {
            require_owned_channel(db_tx, account_id, db_owner, &db_channel_id)?;
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel set public_post = ${bool = v} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        return Ok(channel_row_get(db_tx, account_id, db_owner, &db_channel_id)?.context("No such channel")?);
    }).await.context("Error modifying channel")?;
    return Ok(row.into_res());
}

pub async fn channel_delete(state: &State, account: AccountId, req: ChannelDelete) -> Result<(), loga::Error> {
    let account_id = DbAccountId(account);
    let db_owner = DbIdentity(req.id.identity);
    let db_channel_id = DbChannelId(req.id.channel.clone());
    let now = jiff::Timestamp::now();
    tx(&state.db, move |db_tx| {
        good_ormning::sqlite::good_query!(
            db,
            r#"update channel set deleted = ${time_ms_t = &crate::interface::db::DbTimeMs(now)} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(());
    }).await.context("Error deleting channel")?;
    state.pull_scheduler.remove_channel(account, req.id.identity, &req.id.channel).await;
    return Ok(());
}

pub async fn channel_member_list(
    state: &State,
    account: AccountId,
    req: ChannelMemberList,
) -> Result<Vec<ChannelMemberRes>, loga::Error> {
    let account_id = DbAccountId(account);
    let db_owner = DbIdentity(req.channel.identity);
    let db_channel_id = DbChannelId(req.channel.channel);
    let mut members = tx(&state.db, move |db_tx| {
        return Ok(good_ormning::sqlite::good_query_many!(
            db,
            r#"select member, can_post from channel_member where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?.into_iter().map(|r| ChannelMemberRes {
            identity: r.member.0,
            can_post: r.can_post,
        }).collect::<Vec<_>>());
    }).await.context("Error listing channel members")?;
    members.sort_by(|a, b| a.identity.cmp(&b.identity));
    return Ok(members);
}

pub async fn channel_member_modify(
    state: &State,
    account: AccountId,
    req: ChannelMemberModify,
) -> Result<(), loga::Error> {
    let account_id = DbAccountId(account);
    let db_owner = DbIdentity(req.channel.identity);
    let db_channel_id = DbChannelId(req.channel.channel.clone());
    let db_member = DbIdentity(req.member);
    let changed = tx(&state.db, move |db_tx| {
        require_owned_channel(db_tx, account_id, db_owner, &db_channel_id)?;
        let mut changed = false;
        if let Some(v) = req.can_post {
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel_member set can_post = ${bool = v} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and member = ${identity_id_t = &db_member}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
            if db_tx.0.changes() == 0 {
                return Err(loga::err("No such channel member"));
            }
            changed = true;
        }
        return Ok(changed);
    }).await.context("Error modifying channel member")?;
    if changed {
        let key = PullKey {
            account_id: account,
            owner: req.channel.identity,
            channel: req.channel.channel,
            source: req.member,
        };
        match req.can_post {
            Some(true) => state.pull_scheduler.add_or_reset(key).await,
            Some(false) => state.pull_scheduler.remove(&key).await,
            None => { },
        }
    }
    return Ok(());
}

pub async fn channel_member_delete(
    state: &State,
    account: AccountId,
    req: ChannelMemberDelete,
) -> Result<(), loga::Error> {
    let account_id = DbAccountId(account);
    let db_owner = DbIdentity(req.channel.identity);
    let db_channel_id = DbChannelId(req.channel.channel.clone());
    let db_member = DbIdentity(req.member);
    tx(&state.db, move |db_tx| {
        good_ormning::sqlite::good_query!(
            db,
            r#"delete from channel_member where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and member = ${identity_id_t = &db_member}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(());
    }).await.context("Error deleting channel member")?;
    state.pull_scheduler.remove(&PullKey {
        account_id: account,
        owner: req.channel.identity,
        channel: req.channel.channel,
        source: req.member,
    }).await;
    return Ok(());
}

pub async fn channel_invite_create(
    state: &State,
    account: AccountId,
    req: ChannelInviteCreate,
) -> Result<ChannelInviteRes, loga::Error> {
    let account_id = DbAccountId(account);
    let channel = req.channel.clone();
    let db_owner = DbIdentity(channel.identity);
    let db_channel_id = DbChannelId(channel.channel.clone());
    let idem = req.idem.clone().unwrap_or_default();
    let token = random_token();
    return Ok(tx(&state.db, move |db_tx| {
        require_owned_channel(db_tx, account_id, db_owner, &db_channel_id)?;
        if !idem.is_empty() {
            let existing = good_ormning::sqlite::good_query_opt!(
                db,
                r#"select rowid, token, name, description, shown_name, shown_description, single_use, expiry, can_post from channel_invite where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and idem = ${string = idem.as_str()}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
            if let Some(e) = existing {
                return Ok(ChannelInviteRes {
                    id: ChannelInviteId(i64_to_u64(e.rowid)),
                    token: QualifiedChannelInviteToken {
                        channel: channel,
                        token: ChannelInviteToken(e.token),
                    },
                    name: e.name,
                    description: e.description,
                    shown_name: e.shown_name,
                    shown_description: e.shown_description,
                    single_use: e.single_use,
                    expiry: e.expiry.map(|t| t.0),
                    can_post: e.can_post,
                });
            }
        }
        good_ormning::sqlite::good_query!(
            db,
            r#"insert into channel_invite (account_id, identity, channel, idem, token, name, description, shown_name, shown_description, single_use, expiry, used_count, can_post) values (${account_id_t = &account_id}, ${identity_id_t = &db_owner}, ${channel_id_t = &db_channel_id}, ${string = idem.as_str()}, ${string = token.as_str()}, ${string = req.name.as_str()}, ${string = req.description.as_str()}, ${opt string = req.shown_name.as_deref()}, ${opt string = req.shown_description.as_deref()}, ${bool = req.single_use}, ${opt time_ms_t = req.expiry.map(crate::interface::db::DbTimeMs).as_ref()}, ${i64 = 0}, ${bool = req.can_post})"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(ChannelInviteRes {
            id: ChannelInviteId(i64_to_u64(db_tx.0.last_insert_rowid())),
            token: QualifiedChannelInviteToken {
                channel: channel,
                token: ChannelInviteToken(token),
            },
            name: req.name,
            description: req.description,
            shown_name: req.shown_name,
            shown_description: req.shown_description,
            single_use: req.single_use,
            expiry: req.expiry,
            can_post: req.can_post,
        });
    }).await.context("Error inserting channel invite")?);
}

pub async fn channel_invite_list(
    state: &State,
    account: AccountId,
    _req: ChannelInviteList,
) -> Result<Vec<ChannelInviteRes>, loga::Error> {
    let account_id = DbAccountId(account);
    let rows = tx(&state.db, move |db_tx| {
        return Ok(good_ormning::sqlite::good_query_many!(
            db,
            r#"select rowid, identity, channel, token, name, description, shown_name, shown_description, single_use, expiry, can_post from channel_invite where account_id = ${account_id_t = &account_id} order by rowid"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?);
    }).await.context("Error listing channel invites")?;
    return Ok(rows.into_iter().map(|r| ChannelInviteRes {
        id: ChannelInviteId(i64_to_u64(r.rowid)),
        token: QualifiedChannelInviteToken {
            channel: QualifiedChannelId {
                identity: r.identity.0,
                channel: r.channel.0,
            },
            token: ChannelInviteToken(r.token),
        },
        name: r.name,
        description: r.description,
        shown_name: r.shown_name,
        shown_description: r.shown_description,
        single_use: r.single_use,
        expiry: r.expiry.map(|t| t.0),
        can_post: r.can_post,
    }).collect());
}

pub async fn channel_invite_modify(
    state: &State,
    account: AccountId,
    req: ChannelInviteModify,
) -> Result<ChannelInviteRes, loga::Error> {
    let account_id = DbAccountId(account);
    let rowid = crate::subsystems::messages::u64_to_i64(req.id.0);
    let row = tx(&state.db, move |db_tx| {
        if let Some(v) = &req.name {
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel_invite set name = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(v) = &req.description {
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel_invite set description = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(m) = &req.shown_name {
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel_invite set shown_name = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(m) = &req.shown_description {
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel_invite set shown_description = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(v) = req.single_use {
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel_invite set single_use = ${bool = v} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(m) = req.expiry {
            let v = m.value;
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel_invite set expiry = ${opt time_ms_t = v.map(crate::interface::db::DbTimeMs).as_ref()} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(v) = req.can_post {
            good_ormning::sqlite::good_query!(
                db,
                r#"update channel_invite set can_post = ${bool = v} where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        return Ok(good_ormning::sqlite::good_query_opt!(
            db,
            r#"select rowid, identity, channel, token, name, description, shown_name, shown_description, single_use, expiry, can_post from channel_invite where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?.context("No such channel invite")?);
    }).await.context("Error modifying channel invite")?;
    return Ok(ChannelInviteRes {
        id: ChannelInviteId(i64_to_u64(row.rowid)),
        token: QualifiedChannelInviteToken {
            channel: QualifiedChannelId {
                identity: row.identity.0,
                channel: row.channel.0,
            },
            token: ChannelInviteToken(row.token),
        },
        name: row.name,
        description: row.description,
        shown_name: row.shown_name,
        shown_description: row.shown_description,
        single_use: row.single_use,
        expiry: row.expiry.map(|t| t.0),
        can_post: row.can_post,
    });
}

pub async fn channel_invite_delete(
    state: &State,
    account: AccountId,
    req: ChannelInviteDelete,
) -> Result<(), loga::Error> {
    let account_id = DbAccountId(account);
    let rowid = crate::subsystems::messages::u64_to_i64(req.id.0);
    tx(&state.db, move |db_tx| {
        good_ormning::sqlite::good_query!(
            db,
            r#"delete from channel_invite where account_id = ${account_id_t = &account_id} and rowid = ${i64 = rowid}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(());
    }).await.context("Error deleting channel invite")?;
    return Ok(());
}

pub async fn contact_list(
    state: &State,
    account: AccountId,
    _req: ContactList,
) -> Result<Vec<ContactRes>, loga::Error> {
    let account_id = DbAccountId(account);
    return Ok(tx(&state.db, move |db_tx| {
        let rows = good_ormning::sqlite::good_query_many!(
            db,
            r#"select id, name, description, portrait from contact where account_id = ${account_id_t = &account_id} order by id"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        let mut out = vec![];
        for r in rows {
            out.push(ContactRes {
                id: r.id.0,
                name: r.name,
                description: r.description,
                portrait: file_ref(db_tx, account_id, r.portrait)?,
            });
        }
        return Ok(out);
    }).await.context("Error listing contacts")?);
}

pub async fn contact_modify(
    state: &State,
    account: AccountId,
    req: ContactModify,
) -> Result<ContactRes, loga::Error> {
    let account_id = DbAccountId(account);
    let db_id = DbIdentity(req.id);
    let portrait = resolve_modify_portrait(state, account_id, &req.portrait).await?;
    let row = tx(&state.db, move |db_tx| {
        good_ormning::sqlite::good_query!(
            db,
            r#"insert into contact (account_id, id, name, description) values (${account_id_t = &account_id}, ${identity_id_t = &db_id}, ${string = ""}, ${string = ""}) on conflict do nothing"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        if let Some(v) = &req.name {
            good_ormning::sqlite::good_query!(
                db,
                r#"update contact set name = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_id}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if let Some(v) = &req.description {
            good_ormning::sqlite::good_query!(
                db,
                r#"update contact set description = ${string = v.as_str()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_id}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        if !matches!(portrait, PortraitChange::Keep) {
            let current = good_ormning::sqlite::good_query_opt!(
                db,
                r#"select portrait from contact where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_id}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?.flatten();
            let next = portrait.apply(current);
            good_ormning::sqlite::good_query!(
                db,
                r#"update contact set portrait = ${opt string = next.as_deref()} where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_id}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
        }
        let row = good_ormning::sqlite::good_query_opt!(
            db,
            r#"select id, name, description, portrait from contact where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &db_id}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?.context("Contact insertion failed")?;
        return Ok(ContactRes {
            id: row.id.0,
            name: row.name,
            description: row.description,
            portrait: file_ref(db_tx, account_id, row.portrait)?,
        });
    }).await.context("Error modifying contact")?;
    return Ok(row);
}

pub async fn notification_register(
    state: &State,
    account: AccountId,
    req: NotificationRegister,
) -> Result<(), loga::Error> {
    let account_id = DbAccountId(account);
    let endpoint =
        req
            .data
            .get("endpoint")
            .and_then(|v| v.as_str())
            .context("Push subscription missing 'endpoint' field")?
            .to_string();
    let data = serde_json::to_string(&req.data).unwrap();
    tx(&state.db, move |db_tx| {
        good_ormning::sqlite::good_query!(
            db,
            r#"insert into push_subscription (account_id, endpoint, data) values (${account_id_t = &account_id}, ${string = endpoint.as_str()}, ${string = data.as_str()}) on conflict (account_id, endpoint) do update set data = excluded.data"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(());
    }).await.context("Error registering push subscription")?;
    return Ok(());
}

struct PushResult {
    seq: Option<u64>,
    recipients: Vec<Identity>,
}

pub async fn message_push(state: &Arc<State>, account: AccountId, req: MessagePush) -> Result<(), loga::Error> {
    let account_id = DbAccountId(account);
    let sender = req.identity;
    let owner = req.channel.identity;
    if let MessageRel::EditOf(target) = &req.rel {
        if target.identity != sender {
            return Err(loga::err("Can only edit your own messages"));
        }
    }
    let channel = req.channel.clone();
    let body_text = req.body.clone();
    let result = tx(&state.db, move |db_tx| {
        let db_sender = DbIdentity(sender);
        let db_owner = DbIdentity(owner);
        let db_channel_id = DbChannelId(req.channel.channel.clone());
        let channel_row = good_ormning::sqlite::good_query_opt!(
            db,
            r#"select local_identity, next_sender_unique from channel where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id} and deleted is null"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?.context("No such channel")?;
        if channel_row.local_identity.0 != sender {
            return Err(loga::err("Sender isn't this account's identity in the channel"));
        }
        let existing = good_ormning::sqlite::good_query_opt!(
            db,
            r#"select seq from message where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and sender = ${identity_id_t = &db_sender} and client_id = ${string = req.client_id.0.as_str()}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        if existing.is_some() {
            return Ok(None);
        }
        let existing_outbox = good_ormning::sqlite::good_query_opt!(
            db,
            r#"select sender_unique from outbox where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and sender = ${identity_id_t = &db_sender} and client_id = ${string = req.client_id.0.as_str()}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        if existing_outbox.is_some() {
            return Ok(None);
        }
        let mut signer = load_signer_tx(db_tx, account_id, db_sender)?;
        let sender_unique = channel_row.next_sender_unique;
        let next_sender_unique = sender_unique.checked_add(1).context("sender_unique counter overflow")?;
        good_ormning::sqlite::good_query!(
            db,
            r#"update channel set next_sender_unique = ${i64 = next_sender_unique} where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        let body = MessageBody {
            channel: req.channel.clone(),
            client_id: Some(req.client_id.clone()),
            id: MessageId {
                identity: sender,
                unique: i64_to_u64(sender_unique),
            },
            rel: req.rel.clone(),
            body: req.body.clone(),
        };
        let body_json = serde_json::to_string(&body).unwrap();
        let (_root, _chain, sig) =
            signer.sign(MessageBody::DOMAIN, body_json.as_bytes()).context("Signing MessageBody failed")?;
        let raw = serde_json::to_string(&Message(JsonSignature {
            _p: Default::default(),
            message: body_json,
            signature: BytesZb32(sig),
        })).unwrap();
        if sender == owner {
            let seq =
                ingest_message(
                    db_tx,
                    account_id,
                    db_owner,
                    &db_channel_id,
                    &raw,
                    &body,
                )?.context("Message conflicted with an existing message")?;
            return Ok(Some(PushResult {
                seq: Some(seq),
                recipients: channel_members(db_tx, account_id, db_owner, &db_channel_id)?,
            }));
        } else {
            let created = jiff::Timestamp::now();
            good_ormning::sqlite::good_query!(
                db,
                r#"insert into outbox (account_id, identity, channel, sender, sender_unique, client_id, raw, created) values (${account_id_t = &account_id}, ${identity_id_t = &db_owner}, ${channel_id_t = &db_channel_id}, ${identity_id_t = &db_sender}, ${i64 = sender_unique}, ${string = req.client_id.0.as_str()}, ${string = raw.as_str()}, ${time_ms_t = &crate::interface::db::DbTimeMs(created)})"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
            return Ok(Some(PushResult {
                seq: None,
                recipients: vec![owner],
            }));
        }
    }).await.context("Error storing message")?;
    let Some(result) = result else {
        return Ok(());
    };
    match result.seq {
        Some(seq) => {
            publish_local_notification(state, account, channel.clone(), seq, body_text, false).await;
        },
        None => {
            state
                .ws_bus
                .publish(
                    account,
                    shared::interface::wire::s2c::WsEvent::Outbox(shared::interface::wire::s2c::OutboxChange {
                        channel: channel.clone(),
                        sender: sender,
                    }),
                )
                .await;
        },
    }
    notify_remotes(state, account, sender, channel, result.recipients).await;
    return Ok(());
}

fn insert_joined_channel(
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

pub async fn channel_join_preview(
    state: &Arc<State>,
    _account: AccountId,
    req: ChannelJoinPreview,
) -> Result<ChannelJoinPreviewRes, loga::Error> {
    use crate::interface::s2s::GetJoinPreview;

    let (host, target, _) = join_remote_from_url(&req.url)?;
    let res =
        state
            .s2s_client
            .get_join_preview(host, GetJoinPreview { target: target })
            .await
            .context("s2s join preview failed")?;
    return Ok(ChannelJoinPreviewRes {
        name: res.name,
        description: res.description,
        portrait: res.portrait,
    });
}

pub async fn channel_add(
    state: &Arc<State>,
    account: AccountId,
    req: ChannelAdd,
) -> Result<ChannelRes, loga::Error> {
    use crate::interface::s2s::GetChannelInfo;

    let account_id = DbAccountId(account);
    let local_identity = req.identity;
    let owner = req.channel.identity;
    let channel_id = req.channel.channel.clone();
    if local_identity == owner {
        return Err(loga::err("Can't add a channel with its owning identity"));
    }
    validate_join_name(&req.name)?;
    tx(&state.db, move |db_tx| {
        return require_owned_identity(db_tx, account_id, DbIdentity(local_identity));
    }).await?;
    let portrait =
        resolve_join_portrait(state, account_id, &KwaUrl::Channel(req.channel.clone()), req.portrait).await?;
    let info =
        state
            .s2s_client
            .get_channel_info(owner, None, GetChannelInfo { channel: req.channel.clone() })
            .await
            .context("s2s channel info failed")?;
    let page_size = info.page_size as i64;
    let row = tx(&state.db, {
        let channel_id = channel_id.clone();
        move |db_tx| {
            let db_owner = DbIdentity(owner);
            let db_channel_id = DbChannelId(channel_id.clone());
            let idem = format!("{}:{}", owner, channel_id.0);
            insert_joined_channel(
                db_tx,
                account_id,
                db_owner,
                &db_channel_id,
                DbIdentity(local_identity),
                &idem,
                &req.name,
                &req.description,
                portrait.as_ref().map(|h| h.0.as_str()),
                page_size,
                false,
            )?;
            crate::subsystems::messages::store_channel_info(
                db_tx,
                account_id,
                db_owner,
                &db_channel_id,
                &info,
                true,
            )?;
            return Ok(
                channel_row_get(db_tx, account_id, db_owner, &db_channel_id)?.context("Added channel vanished")?,
            );
        }
    }).await.context("Error inserting added channel row")?;
    state.pull_scheduler.trigger_immediate(PullKey {
        account_id: account,
        owner: owner,
        channel: channel_id,
        source: owner,
    }).await;
    return Ok(row.into_res());
}

pub async fn channel_join_channel(
    state: &Arc<State>,
    account: AccountId,
    req: ChannelJoinChannel,
) -> Result<ChannelRes, loga::Error> {
    use crate::interface::s2s::{
        ChannelOrIdentity,
        InvitationToken,
        Join,
    };

    let account_id = DbAccountId(account);
    let sender = req.sender;
    let owner = req.channel.identity;
    validate_join_name(&req.name)?;
    let portrait = resolve_join_portrait(state, account_id, &KwaUrl::ChannelInvite(KwaUrlChannelInvite {
        channel: req.channel.clone(),
        code: req.code.clone(),
    }), req.portrait).await?;
    let signer = crate::subsystems::messages::load_signer(state, account, sender).await?;
    let returned = state.s2s_client.join(owner, &signer, Join {
        target: ChannelOrIdentity::Channel(req.channel.channel.clone()),
        invitation: InvitationToken::Channel(ChannelInviteToken(req.code)),
    }).await.context("s2s Join failed")?;
    let channel_id = returned.channel;
    let info = returned.info;
    let page_size = info.page_size as i64;
    let row = tx(&state.db, {
        let channel_id = channel_id.clone();
        move |db_tx| {
            let db_owner = DbIdentity(owner);
            let db_channel_id = DbChannelId(channel_id.clone());
            let idem = format!("{}:{}", owner, channel_id.0);
            insert_joined_channel(
                db_tx,
                account_id,
                db_owner,
                &db_channel_id,
                DbIdentity(sender),
                &idem,
                &req.name,
                &req.description,
                portrait.as_ref().map(|h| h.0.as_str()),
                page_size,
                true,
            )?;
            good_ormning::sqlite::good_query!(
                db,
                r#"insert into contact (account_id, id, name, description, portrait) values (${account_id_t = &account_id}, ${identity_id_t = &db_owner}, ${string = req.name.as_str()}, ${string = req.description.as_str()}, ${opt string = portrait.as_ref().map(|h| h.0.as_str())}) on conflict (account_id, id) do nothing"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
            crate::subsystems::messages::store_channel_info(
                db_tx,
                account_id,
                db_owner,
                &db_channel_id,
                &info,
                true,
            )?;
            return Ok(
                channel_row_get(db_tx, account_id, db_owner, &db_channel_id)?.context("Joined channel vanished")?,
            );
        }
    }).await.context("Error inserting joined channel row")?;
    state.pull_scheduler.trigger_immediate(PullKey {
        account_id: account,
        owner: owner,
        channel: channel_id,
        source: owner,
    }).await;
    return Ok(row.into_res());
}

pub async fn channel_join_identity(
    state: &Arc<State>,
    account: AccountId,
    req: ChannelJoinIdentity,
) -> Result<ChannelRes, loga::Error> {
    use crate::interface::s2s::{
        ChannelOrIdentity,
        InvitationToken,
        Join,
    };

    let account_id = DbAccountId(account);
    let sender = req.sender;
    let target = req.identity;
    validate_join_name(&req.name)?;
    let portrait = resolve_join_portrait(state, account_id, &KwaUrl::IdentityInvite(KwaUrlIdentityInvite {
        identity: target,
        code: req.code.clone(),
    }), req.portrait).await?;
    let signer = crate::subsystems::messages::load_signer(state, account, sender).await?;
    let returned = state.s2s_client.join(target, &signer, Join {
        target: ChannelOrIdentity::Identity(target),
        invitation: InvitationToken::Identity(IdentityInviteToken(req.code)),
    }).await.context("s2s Join (identity) failed")?;
    let channel_id = returned.channel;
    let info = returned.info;
    let page_size = info.page_size as i64;
    let row = tx(&state.db, {
        let channel_id = channel_id.clone();
        move |db_tx| {
            let db_target = DbIdentity(target);
            let db_channel_id = DbChannelId(channel_id.clone());
            good_ormning::sqlite::good_query!(
                db,
                r#"insert into contact (account_id, id, name, description, portrait) values (${account_id_t = &account_id}, ${identity_id_t = &db_target}, ${string = req.name.as_str()}, ${string = req.description.as_str()}, ${opt string = portrait.as_ref().map(|h| h.0.as_str())}) on conflict (account_id, id) do update set name = excluded.name, description = excluded.description, portrait = excluded.portrait"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
            let idem = format!("dm:{}:{}", target, channel_id.0);
            insert_joined_channel(
                db_tx,
                account_id,
                db_target,
                &db_channel_id,
                DbIdentity(sender),
                &idem,
                &req.name,
                &req.description,
                portrait.as_ref().map(|h| h.0.as_str()),
                page_size,
                true,
            )?;
            crate::subsystems::messages::store_channel_info(
                db_tx,
                account_id,
                db_target,
                &db_channel_id,
                &info,
                true,
            )?;
            return Ok(
                channel_row_get(db_tx, account_id, db_target, &db_channel_id)?.context("Joined channel vanished")?,
            );
        }
    }).await.context("Error inserting DM channel after identity join")?;
    state.pull_scheduler.trigger_immediate(PullKey {
        account_id: account,
        owner: target,
        channel: channel_id,
        source: target,
    }).await;
    return Ok(row.into_res());
}
