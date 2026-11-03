use {
    crate::{
        db,
        dbutil::tx,
        interface::{
            db::{
                DbAccountId,
                DbChannelId,
                DbIdentity,
            },
            s2s::{
                ChannelInfoRes,
                GetActivity,
                FileAuth,
                GetChannelInfo,
                GetFile,
                GetJoinPreview,
                GetMessageHistory,
                GetSnapLookup,
                JoinPreviewRes,
                JoinPreviewTarget,
                GetSnapPage,
                GetSnapPageContainingTime,
                Notify,
                NotifyTimestamp,
                PullOutbox,
                PulledMessage,
                PulledMessages,
                SnapLookupBy,
                SnapLookupRes,
            },
        },
        subsystems::{
            pull_scheduler::PullKey,
            s2s_client::Signer,
            s2s_error::{
                not_found,
                s2s_tx,
                CachePolicy,
                Cached,
                S2sError,
                SNAP_PAGE_MAX_AGE_SECS,
            },
        },
        State,
    },
    jiff::Timestamp,
    loga::{
        ea,
        ResultContext,
    },
    rusqlite::Transaction,
    shared::interface::{
        shared::{
            AccountId,
            MessageBody,
            MessageId,
            MessageRel,
            QualifiedChannelId,
        },
        wire::{
            c2s::ActivityOffset,
            s2c::{
                Notification,
                WsEvent,
            },
        },
    },
    spaghettinuum::interface::identity::{
        Identity,
        IdentitySecret,
    },
    spaghettinuum_native::utils::identity_secret::{
        IdentitySigner,
        LocalIdentitySigner,
    },
    std::{
        collections::HashMap,
        sync::{
            Arc,
            Mutex,
        },
        time::Duration,
    },
};

pub const PULL_BATCH_LIMIT: u32 = 100;
pub const BACKFILL_INTERVAL: Duration = Duration::from_secs(2);
pub const RETENTION_CLEANUP_INTERVAL: Duration = Duration::from_secs(3600);

pub struct RelCols {
    pub rel_type: Option<String>,
    pub rel_target_sender: Option<DbIdentity>,
    pub rel_target_unique: Option<i64>,
}

pub const REL_REPLY_TO: &str = "reply_to";
pub const REL_EDIT_OF: &str = "edit_of";

pub fn rel_cols(rel: &MessageRel) -> RelCols {
    match rel {
        MessageRel::None => {
            return RelCols {
                rel_type: None,
                rel_target_sender: None,
                rel_target_unique: None,
            };
        },
        MessageRel::ReplyTo(mid) => {
            return RelCols {
                rel_type: Some(REL_REPLY_TO.to_string()),
                rel_target_sender: Some(DbIdentity(mid.identity)),
                rel_target_unique: Some(u64_to_i64(mid.unique)),
            };
        },
        MessageRel::EditOf(mid) => {
            return RelCols {
                rel_type: Some(REL_EDIT_OF.to_string()),
                rel_target_sender: Some(DbIdentity(mid.identity)),
                rel_target_unique: Some(u64_to_i64(mid.unique)),
            };
        },
    }
}

pub fn i64_to_u64(v: i64) -> u64 {
    return u64::from_ne_bytes(v.to_ne_bytes());
}

pub fn u64_to_i64(v: u64) -> i64 {
    return i64::from_ne_bytes(v.to_ne_bytes());
}

pub fn retention_cutoff(now: Timestamp, retention_secs: Option<i64>) -> Result<Option<Timestamp>, loga::Error> {
    let Some(secs) = retention_secs else {
        return Ok(None);
    };
    return Ok(
        Some(
            now
                .checked_sub(Duration::from_secs(secs.max(0) as u64))
                .context_with("Retention cutoff out of range", ea!(retention_secs = secs))?,
        ),
    );
}

#[derive(Clone)]
pub struct LocalChannel {
    pub local_identity: Identity,
    pub page_size: usize,
    pub authenticated: bool,
    pub public_read: bool,
    pub public_post: bool,
    pub retention_secs: Option<i64>,
    pub latest_seq: Option<i64>,
    pub latest_snap_offset: Option<i64>,
    pub floor_snap_offset: Option<i64>,
    pub backfill_done: bool,
}

impl LocalChannel {
    pub fn is_owner(&self, owner: Identity) -> bool {
        return self.local_identity == owner;
    }
}

pub fn local_channel_get(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
) -> Result<Option<LocalChannel>, loga::Error> {
    let row = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select local_identity, page_size, authenticated, public_read, public_post, retention_secs, latest_seq, latest_snap_offset, floor_snap_offset, backfill_done from channel where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and id = ${channel_id_t = channel} and deleted is null"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    return Ok(row.map(|r| LocalChannel {
        local_identity: r.local_identity.0,
        page_size: r.page_size.max(1) as usize,
        authenticated: r.authenticated,
        public_read: r.public_read,
        public_post: r.public_post,
        retention_secs: r.retention_secs,
        latest_seq: r.latest_seq,
        latest_snap_offset: r.latest_snap_offset,
        floor_snap_offset: r.floor_snap_offset,
        backfill_done: r.backfill_done,
    }));
}

pub fn hosted_identity_account(
    db_tx: &mut db::Db<Transaction<'_>>,
    identity: DbIdentity,
) -> Result<Option<DbAccountId>, loga::Error> {
    return Ok(good_ormning::sqlite::good_query_opt!(
        db,
        r#"select account_id from identity where id = ${identity_id_t = &identity} and soft_deleted_at is null"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?);
}

pub fn owner_channel_get(
    db_tx: &mut db::Db<Transaction<'_>>,
    owner: DbIdentity,
    channel: &DbChannelId,
) -> Result<Option<(DbAccountId, LocalChannel)>, loga::Error> {
    let Some(account) = hosted_identity_account(db_tx, owner)? else {
        return Ok(None);
    };
    let Some(local) = local_channel_get(db_tx, account, owner, channel)? else {
        return Ok(None);
    };
    if !local.is_owner(owner.0) {
        return Ok(None);
    }
    return Ok(Some((account, local)));
}

pub fn is_channel_member(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    member: DbIdentity,
) -> Result<bool, loga::Error> {
    return Ok(good_ormning::sqlite::good_query_opt!(
        db,
        r#"select 1 as x from channel_member where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and member = ${identity_id_t = &member}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?.is_some());
}

pub fn member_can_post(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    member: DbIdentity,
) -> Result<bool, loga::Error> {
    return Ok(good_ormning::sqlite::good_query_opt!(
        db,
        r#"select can_post from channel_member where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and member = ${identity_id_t = &member}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?.unwrap_or(false));
}

pub fn channel_members(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
) -> Result<Vec<Identity>, loga::Error> {
    return Ok(good_ormning::sqlite::good_query_many!(
        db,
        r#"select member from channel_member where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?.into_iter().map(|r| r.0).collect());
}

pub fn posting_members(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
) -> Result<Vec<Identity>, loga::Error> {
    return Ok(good_ormning::sqlite::good_query_many!(
        db,
        r#"select member from channel_member where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and can_post = ${bool = true}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?.into_iter().map(|r| r.0).collect());
}

pub fn check_read_access(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    local: &LocalChannel,
    caller: Option<Identity>,
) -> Result<(), S2sError> {
    if local.public_read {
        return Ok(());
    }
    let Some(caller) = caller else {
        return Err(S2sError::Unauthorized("Channel requires authentication for reads".to_string()));
    };
    if caller == owner.0 || is_channel_member(db_tx, account, owner, channel, DbIdentity(caller))? {
        return Ok(());
    }
    return Err(S2sError::Forbidden("Caller is not a member of this channel".to_string()));
}

pub struct MsgRow {
    pub sender: Identity,
    pub sender_unique: i64,
    pub seq: i64,
    pub snap_offset: Option<i64>,
    pub receive_time: Timestamp,
    pub client_id: Option<String>,
    pub raw: String,
    pub body_text: String,
    pub rel_type: Option<String>,
    pub rel_target_sender: Option<Identity>,
    pub rel_target_unique: Option<i64>,
}

impl MsgRow {
    pub fn from_verified(v: &Verified) -> MsgRow {
        let rel = rel_cols(&v.body.rel);
        return MsgRow {
            sender: v.body.id.identity,
            sender_unique: u64_to_i64(v.body.id.unique),
            seq: u64_to_i64(v.seq),
            snap_offset: v.snap_offset.map(u64_to_i64),
            receive_time: v.receive_time,
            client_id: v.body.client_id.as_ref().map(|c| c.0.clone()),
            raw: v.raw.clone(),
            body_text: v.body.body.clone(),
            rel_type: rel.rel_type,
            rel_target_sender: rel.rel_target_sender.map(|s| s.0),
            rel_target_unique: rel.rel_target_unique,
        };
    }

    pub fn to_pulled(&self) -> Result<PulledMessage, loga::Error> {
        return Ok(PulledMessage {
            seq: i64_to_u64(self.seq),
            snap_offset: self.snap_offset.map(i64_to_u64),
            receive_time: self.receive_time,
            message: serde_json::from_str(&self.raw).context("Stored message was not valid JSON")?,
        });
    }
}

pub fn snap_page_rows(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    page_size: usize,
    page: u64,
) -> Result<(Vec<MsgRow>, Vec<MsgRow>), loga::Error> {
    let start = u64_to_i64(page * page_size as u64);
    let end = start + page_size as i64;
    let originals = good_ormning::sqlite::good_query_many!(
        db,
        r#"select sender, sender_unique, seq, snap_offset, receive_time, client_id, raw, body_text, rel_type, rel_target_sender, rel_target_unique from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and snap_offset >= ${i64 = start} and snap_offset < ${i64 = end} order by snap_offset"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    let mut out_originals = Vec::with_capacity(originals.len());
    let mut out_edits = vec![];
    for r in originals {
        let edit = good_ormning::sqlite::good_query_opt!(
            db,
            r#"select sender, sender_unique, seq, snap_offset, receive_time, client_id, raw, body_text, rel_type, rel_target_sender, rel_target_unique from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and rel_type = ${string = REL_EDIT_OF} and rel_target_sender = ${identity_id_t = &r.sender} and rel_target_unique = ${i64 = r.sender_unique} order by seq desc limit ${i64 = 1}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        if let Some(e) = edit {
            out_edits.push(MsgRow {
                sender: e.sender.0,
                sender_unique: e.sender_unique,
                seq: e.seq,
                snap_offset: e.snap_offset,
                receive_time: e.receive_time.0,
                client_id: e.client_id,
                raw: e.raw,
                body_text: e.body_text,
                rel_type: e.rel_type,
                rel_target_sender: e.rel_target_sender.map(|s| s.0),
                rel_target_unique: e.rel_target_unique,
            });
        }
        out_originals.push(MsgRow {
            sender: r.sender.0,
            sender_unique: r.sender_unique,
            seq: r.seq,
            snap_offset: r.snap_offset,
            receive_time: r.receive_time.0,
            client_id: r.client_id,
            raw: r.raw,
            body_text: r.body_text,
            rel_type: r.rel_type,
            rel_target_sender: r.rel_target_sender.map(|s| s.0),
            rel_target_unique: r.rel_target_unique,
        });
    }
    return Ok((out_originals, out_edits));
}

pub fn message_history_rows(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    id: &MessageId,
) -> Result<Option<Vec<MsgRow>>, loga::Error> {
    let db_sender = DbIdentity(id.identity);
    let sender_unique = u64_to_i64(id.unique);
    let Some(original) = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select sender, sender_unique, seq, snap_offset, receive_time, client_id, raw, body_text, rel_type, rel_target_sender, rel_target_unique from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and sender = ${identity_id_t = &db_sender} and sender_unique = ${i64 = sender_unique} and snap_offset is not null"#;
        db_tx
    ).map_err(|e| loga::err(e.0))? else {
        return Ok(None);
    };
    let edits = good_ormning::sqlite::good_query_many!(
        db,
        r#"select sender, sender_unique, seq, snap_offset, receive_time, client_id, raw, body_text, rel_type, rel_target_sender, rel_target_unique from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and rel_type = ${string = REL_EDIT_OF} and rel_target_sender = ${identity_id_t = &db_sender} and rel_target_unique = ${i64 = sender_unique} order by seq"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    let mut out = Vec::with_capacity(edits.len() + 1);
    out.push(MsgRow {
        sender: original.sender.0,
        sender_unique: original.sender_unique,
        seq: original.seq,
        snap_offset: original.snap_offset,
        receive_time: original.receive_time.0,
        client_id: original.client_id,
        raw: original.raw,
        body_text: original.body_text,
        rel_type: original.rel_type,
        rel_target_sender: original.rel_target_sender.map(|s| s.0),
        rel_target_unique: original.rel_target_unique,
    });
    for e in edits {
        out.push(MsgRow {
            sender: e.sender.0,
            sender_unique: e.sender_unique,
            seq: e.seq,
            snap_offset: e.snap_offset,
            receive_time: e.receive_time.0,
            client_id: e.client_id,
            raw: e.raw,
            body_text: e.body_text,
            rel_type: e.rel_type,
            rel_target_sender: e.rel_target_sender.map(|s| s.0),
            rel_target_unique: e.rel_target_unique,
        });
    }
    return Ok(Some(out));
}

pub fn activity_rows(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    start: i64,
    end: i64,
) -> Result<Vec<(i64, String)>, loga::Error> {
    return Ok(good_ormning::sqlite::good_query_many!(
        db,
        r#"select seq, raw from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and seq >= ${i64 = start} and seq < ${i64 = end} order by seq"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?.into_iter().map(|r| (r.seq, r.raw)).collect());
}

pub struct SnapPosition {
    pub original_id: MessageId,
    pub snap_offset: i64,
    pub receive_time: Timestamp,
}

pub fn snap_position_by_id(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    id: &MessageId,
) -> Result<Option<SnapPosition>, loga::Error> {
    let db_sender = DbIdentity(id.identity);
    let sender_unique = u64_to_i64(id.unique);
    let Some(row) = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select rel_type, rel_target_sender, rel_target_unique from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and sender = ${identity_id_t = &db_sender} and sender_unique = ${i64 = sender_unique}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))? else {
        return Ok(None);
    };
    let (origin_sender, origin_unique) = match row.rel_type.as_deref() {
        Some(REL_EDIT_OF) => (
            row.rel_target_sender.context("edit_of row missing rel_target_sender")?,
            row.rel_target_unique.context("edit_of row missing rel_target_unique")?,
        ),
        _ => (db_sender, sender_unique),
    };
    let Some(origin) = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select receive_time, snap_offset from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and sender = ${identity_id_t = &origin_sender} and sender_unique = ${i64 = origin_unique}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))? else {
        return Ok(None);
    };
    return Ok(Some(SnapPosition {
        original_id: MessageId {
            identity: origin_sender.0,
            unique: i64_to_u64(origin_unique),
        },
        snap_offset: origin.snap_offset.context("Edit target is itself an edit")?,
        receive_time: origin.receive_time.0,
    }));
}

pub fn snap_position_by_client_id(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    sender: Identity,
    client_id: &str,
) -> Result<Option<SnapPosition>, loga::Error> {
    let db_sender = DbIdentity(sender);
    let Some(row) = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select sender_unique from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and sender = ${identity_id_t = &db_sender} and client_id = ${string = client_id}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))? else {
        return Ok(None);
    };
    return snap_position_by_id(db_tx, account, owner, channel, &MessageId {
        identity: sender,
        unique: i64_to_u64(row),
    });
}

pub fn snap_offset_containing_time(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    time: Timestamp,
) -> Result<Option<i64>, loga::Error> {
    let row = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select snap_offset from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and snap_offset is not null and receive_time <= ${time_ms_t = &crate::interface::db::DbTimeMs(time)} order by snap_offset desc limit ${i64 = 1}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    if let Some(Some(offset)) = row {
        return Ok(Some(offset));
    }
    let row = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select snap_offset from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and snap_offset is not null and receive_time > ${time_ms_t = &crate::interface::db::DbTimeMs(time)} order by snap_offset asc limit ${i64 = 1}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    let Some(Some(offset)) = row else {
        return Ok(None);
    };
    return Ok(Some(offset));
}

pub fn earliest_snap_offset(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
) -> Result<Option<i64>, loga::Error> {
    let row = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select snap_offset from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and snap_offset is not null order by snap_offset asc limit ${i64 = 1}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    let Some(Some(offset)) = row else {
        return Ok(None);
    };
    return Ok(Some(offset));
}

pub fn ingest_message(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    raw: &str,
    body: &MessageBody,
) -> Result<Option<u64>, loga::Error> {
    let Some(local) = local_channel_get(db_tx, account, owner, channel)? else {
        return Ok(None);
    };
    let seq = local.latest_seq.map(|s| s + 1).unwrap_or(0);
    let rel = rel_cols(&body.rel);
    let is_edit = matches!(body.rel, MessageRel::EditOf(_));
    let snap_offset = if is_edit {
        None
    } else {
        Some(local.latest_snap_offset.map(|s| s + 1).unwrap_or(0))
    };
    let sender = DbIdentity(body.id.identity);
    let sender_unique = u64_to_i64(body.id.unique);
    let client_id = body.client_id.as_ref().map(|c| c.0.clone());
    let receive_time = Timestamp::now();
    good_ormning::sqlite::good_query!(
        db,
        r#"insert into message (account_id, identity, channel, sender, sender_unique, seq, snap_offset, receive_time, client_id, raw, body_text, rel_type, rel_target_sender, rel_target_unique) values (${account_id_t = &account}, ${identity_id_t = &owner}, ${channel_id_t = channel}, ${identity_id_t = &sender}, ${i64 = sender_unique}, ${i64 = seq}, ${opt i64 = snap_offset}, ${time_ms_t = &crate::interface::db::DbTimeMs(receive_time)}, ${opt string = client_id.as_deref()}, ${string = raw}, ${string = body.body.as_str()}, ${opt string = rel.rel_type.as_deref()}, ${opt identity_id_t = rel.rel_target_sender.as_ref()}, ${opt i64 = rel.rel_target_unique}) on conflict do nothing"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    if db_tx.0.changes() == 0 {
        return Ok(None);
    }
    let latest_snap_offset = snap_offset.or(local.latest_snap_offset);
    good_ormning::sqlite::good_query!(
        db,
        r#"update channel set latest_seq = ${opt i64 = Some(seq)}, latest_snap_offset = ${opt i64 = latest_snap_offset} where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and id = ${channel_id_t = channel}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    return Ok(Some(i64_to_u64(seq)));
}

pub fn store_remote_message(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    local_identity: Identity,
    v: &Verified,
    cutoff: Option<Timestamp>,
) -> Result<bool, loga::Error> {
    if let Some(cutoff) = cutoff {
        if v.receive_time < cutoff {
            return Ok(false);
        }
    }
    let rel = rel_cols(&v.body.rel);
    let sender = DbIdentity(v.body.id.identity);
    let sender_unique = u64_to_i64(v.body.id.unique);
    let seq = u64_to_i64(v.seq);
    let snap_offset = v.snap_offset.map(u64_to_i64);
    let client_id = v.body.client_id.as_ref().map(|c| c.0.clone());
    good_ormning::sqlite::good_query!(
        db,
        r#"insert into message (account_id, identity, channel, sender, sender_unique, seq, snap_offset, receive_time, client_id, raw, body_text, rel_type, rel_target_sender, rel_target_unique) values (${account_id_t = &account}, ${identity_id_t = &owner}, ${channel_id_t = channel}, ${identity_id_t = &sender}, ${i64 = sender_unique}, ${i64 = seq}, ${opt i64 = snap_offset}, ${time_ms_t = &crate::interface::db::DbTimeMs(v.receive_time)}, ${opt string = client_id.as_deref()}, ${string = v.raw.as_str()}, ${string = v.body.body.as_str()}, ${opt string = rel.rel_type.as_deref()}, ${opt identity_id_t = rel.rel_target_sender.as_ref()}, ${opt i64 = rel.rel_target_unique}) on conflict do nothing"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    let inserted = db_tx.0.changes() > 0;
    if v.body.id.identity == local_identity {
        good_ormning::sqlite::good_query!(
            db,
            r#"delete from outbox where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and sender = ${identity_id_t = &sender} and sender_unique = ${i64 = sender_unique}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
    }
    return Ok(inserted);
}

pub fn load_signer_tx(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    identity: DbIdentity,
) -> Result<LocalIdentitySigner, loga::Error> {
    let secret = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select secret from identity where account_id = ${account_id_t = &account} and id = ${identity_id_t = &identity} and soft_deleted_at is null"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?.context_with("Identity secret not found", ea!(identity = identity.0))?;
    return local_signer(secret.0);
}

pub fn local_signer(secret: IdentitySecret) -> Result<LocalIdentitySigner, loga::Error> {
    match secret {
        IdentitySecret::V1(s) => match s.secret {
            spaghettinuum::interface::identity::v1::Secret::Local(k) => {
                return Ok(LocalIdentitySigner::new(s.public_key_chain, k));
            },
            _ => return Err(loga::err("Non-local identity secrets are not supported")),
        },
    }
}

pub async fn load_signer(state: &State, account_id: AccountId, identity: Identity) -> Result<Signer, loga::Error> {
    let signer = tx(&state.db, move |db_tx| {
        return load_signer_tx(db_tx, DbAccountId(account_id), DbIdentity(identity));
    }).await?;
    return Ok(Arc::new(Mutex::new(signer)) as Arc<Mutex<dyn IdentitySigner>>);
}

pub async fn channel_signer(
    state: &State,
    account_id: AccountId,
    local: &LocalChannel,
) -> Result<Option<Signer>, loga::Error> {
    if !local.authenticated {
        return Ok(None);
    }
    return Ok(Some(load_signer(state, account_id, local.local_identity).await?));
}

pub async fn publish_local_notification(
    state: &Arc<State>,
    account_id: AccountId,
    channel: QualifiedChannelId,
    seq: u64,
    body: String,
    web_push: bool,
) {
    let notification = Notification {
        channel: channel,
        offset: ActivityOffset(seq as usize),
        body: body,
    };
    state.ws_bus.publish(account_id, WsEvent::Activity(notification.clone())).await;
    if web_push {
        let state = state.clone();
        tokio::spawn(async move {
            let payload = serde_json::to_vec(&notification).unwrap();
            if let Err(e) = crate::subsystems::push::send_push_to_account(&state, account_id, &payload).await {
                state.log.log_err(loga::WARN, e.context("Push delivery failed"));
            }
        });
    }
}

pub async fn notify_remotes(
    state: &Arc<State>,
    account_id: AccountId,
    local_identity: Identity,
    channel: QualifiedChannelId,
    recipients: Vec<Identity>,
) {
    if recipients.is_empty() {
        return;
    }
    let signer = match load_signer(state, account_id, local_identity).await {
        Ok(s) => s,
        Err(e) => {
            state.log.log_err(loga::WARN, e.context("Failed to load signer for s2s notify"));
            return;
        },
    };
    for recipient in recipients {
        if recipient == local_identity {
            continue;
        }
        let client = state.s2s_client.clone();
        let signer = signer.clone();
        let channel = channel.clone();
        let log = state.log.clone();
        tokio::spawn(async move {
            if let Err(e) = client.notify(recipient, &signer, channel.identity, channel.channel).await {
                log.log_err(loga::WARN, e.context_with("Failed to send s2s notify", ea!(recipient = recipient)));
            }
        });
    }
}

#[derive(Clone)]
pub struct Verified {
    pub raw: String,
    pub body: MessageBody,
    pub seq: u64,
    pub snap_offset: Option<u64>,
    pub receive_time: Timestamp,
}

pub async fn verify_pulled(
    state: &Arc<State>,
    channel: &QualifiedChannelId,
    messages: Vec<PulledMessage>,
    only_sender: Option<Identity>,
) -> Result<Vec<Verified>, loga::Error> {
    let mut out = Vec::with_capacity(messages.len());
    for pulled in messages {
        let m = pulled.message;
        let unverified_body = match serde_json::from_str::<MessageBody>(&m.0.message) {
            Ok(b) => b,
            Err(e) => {
                state
                    .log
                    .log_err(
                        loga::WARN,
                        loga::err(e.to_string()).context("Skipping pulled message: invalid body JSON"),
                    );
                continue;
            },
        };
        let sender = unverified_body.id.identity;
        if let Some(only) = only_sender {
            if sender != only {
                state
                    .log
                    .log_with(
                        loga::WARN,
                        "Skipping pulled message: host served another sender's message",
                        ea!(sender = sender),
                    );
                continue;
            }
        }
        let announcement =
            match state
                .s2s_state
                .resolver()
                .get(&sender, vec![])
                .await
                .context_with("Failed to resolve sender announcement", ea!(sender = sender))? {
                Some(kvs) => kvs.announcement,
                None => {
                    state
                        .log
                        .log_with(
                            loga::WARN,
                            "Skipping pulled message: sender has no announcement",
                            ea!(sender = sender),
                        );
                    continue;
                },
            };
        let body = match m.0.verify(&sender, &announcement.public_key_root, &announcement.public_key_chain) {
            Ok(b) => b,
            Err(e) => {
                state
                    .log
                    .log_with(loga::WARN, "Skipping pulled message: signature verification failed", ea!(err = e));
                continue;
            },
        };
        if body.channel != *channel {
            state
                .log
                .log_with(
                    loga::WARN,
                    "Skipping pulled message: signed for a different channel",
                    ea!(sender = sender),
                );
            continue;
        }
        if let MessageRel::EditOf(target) = &body.rel {
            if target.identity != sender {
                state
                    .log
                    .log_with(
                        loga::WARN,
                        "Skipping pulled edit: target has a different sender",
                        ea!(sender = sender),
                    );
                continue;
            }
        }
        out.push(Verified {
            raw: serde_json::to_string(&m).unwrap(),
            body: body,
            seq: pulled.seq,
            snap_offset: pulled.snap_offset,
            receive_time: pulled.receive_time,
        });
    }
    return Ok(out);
}

async fn resolve_caller(state: &Arc<State>, token: &str) -> Result<Option<Identity>, S2sError> {
    if token.is_empty() {
        return Ok(None);
    }
    match state.s2s_state.resolve_token(token).await {
        Some(identity) => return Ok(Some(identity)),
        None => return Err(S2sError::Unauthorized("Request token is invalid or expired".to_string())),
    }
}

async fn require_caller(state: &Arc<State>, token: &str) -> Result<Identity, S2sError> {
    match resolve_caller(state, token).await? {
        Some(identity) => return Ok(identity),
        None => return Err(S2sError::Unauthorized("Request requires authentication".to_string())),
    }
}

pub async fn handle_notify(state: &Arc<State>, req: Notify) -> Result<(), S2sError> {
    let signer = req.signer;
    let unverified = match serde_json::from_str::<NotifyTimestamp>(&req.signed.message) {
        Ok(v) => v,
        Err(e) => return Err(S2sError::BadRequest(format!("Notify body is not valid JSON: {}", e))),
    };
    let owner = unverified.owner;
    let channel_id = unverified.channel.clone();
    let local_rows = s2s_tx(&state.db, "looking up local channel for notify", {
        let db_owner = DbIdentity(owner);
        let db_channel_id = DbChannelId(channel_id.clone());
        move |db_tx| {
            return Ok(good_ormning::sqlite::good_query_many!(
                db,
                r#"select account_id, local_identity, public_post from channel where identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id} and deleted is null"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?);
        }
    }).await?;
    if local_rows.is_empty() {
        return Ok(());
    }
    let Some(announcement) =
        state
            .s2s_state
            .resolver()
            .get(&signer, vec![])
            .await
            .context("Error resolving notify signer announcement")?
            .map(|kvs| kvs.announcement) else {
            return Err(S2sError::BadRequest("Notify signer has no announcement".to_string()));
        };
    let ts_body = match req.signed.verify(&signer, &announcement.public_key_root, &announcement.public_key_chain) {
        Ok(v) => v,
        Err(e) => return Err(S2sError::Unauthorized(format!("Notify signature verification failed: {}", e))),
    };
    let now = Timestamp::now();
    let age = now.as_second() - ts_body.timestamp.as_second();
    if age > crate::interface::s2s::NOTIFY_MAX_AGE_SECS || age < -crate::interface::s2s::NOTIFY_MAX_AGE_SECS {
        return Err(S2sError::BadRequest(format!("Notify timestamp is outside the freshness window ({}s)", age)));
    }
    if ts_body.owner != owner || ts_body.channel != channel_id {
        return Err(S2sError::BadRequest("Verified notify body doesn't match parsed body".to_string()));
    }
    let (scheduled, one_shot) = s2s_tx(&state.db, "checking notify signer access", {
        let channel_id = channel_id.clone();
        move |db_tx| {
            let db_owner = DbIdentity(owner);
            let db_channel_id = DbChannelId(channel_id.clone());
            let mut scheduled = vec![];
            let mut one_shot = vec![];
            for row in local_rows {
                let key = PullKey {
                    account_id: row.account_id.0,
                    owner: owner,
                    channel: channel_id.clone(),
                    source: signer,
                };
                if row.local_identity.0 == owner {
                    if member_can_post(db_tx, row.account_id, db_owner, &db_channel_id, DbIdentity(signer))? {
                        scheduled.push(key);
                    } else if row.public_post {
                        one_shot.push(key);
                    }
                } else if signer == owner {
                    scheduled.push(key);
                }
            }
            return Ok((scheduled, one_shot));
        }
    }).await?;
    if scheduled.is_empty() && one_shot.is_empty() {
        return Err(S2sError::Forbidden("Notify signer is not a participant of the referenced channel".to_string()));
    }
    for key in scheduled {
        state.pull_scheduler.trigger_immediate(key).await;
    }
    for key in one_shot {
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = pull_and_store(&state, &key).await {
                state.log.log_err(loga::WARN, e.context("Error pulling from public poster"));
            }
        });
    }
    return Ok(());
}

pub fn channel_info_res(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    local: &LocalChannel,
) -> Result<ChannelInfoRes, loga::Error> {
    return Ok(ChannelInfoRes {
        page_size: u32::try_from(local.page_size).unwrap_or(u32::MAX),
        latest_seq: local.latest_seq.map(i64_to_u64),
        latest_snap_offset: local.latest_snap_offset.map(i64_to_u64),
        earliest_snap_offset: earliest_snap_offset(db_tx, account, owner, channel)?.map(i64_to_u64),
    });
}

pub async fn handle_channel_info(
    state: &Arc<State>,
    token: &str,
    req: GetChannelInfo,
) -> Result<Cached<ChannelInfoRes>, S2sError> {
    let caller = resolve_caller(state, token).await?;
    let db_owner = DbIdentity(req.channel.identity);
    let db_channel_id = DbChannelId(req.channel.channel);
    return s2s_tx(&state.db, "reading channel info", move |db_tx| {
        let (account, local) =
            not_found(owner_channel_get(db_tx, db_owner, &db_channel_id)?, "Channel not found on this server")?;
        check_read_access(db_tx, account, db_owner, &db_channel_id, &local, caller)?;
        return Ok(Cached::no_store(channel_info_res(db_tx, account, db_owner, &db_channel_id, &local)?));
    }).await;
}

pub async fn handle_get_activity(
    state: &Arc<State>,
    token: &str,
    req: GetActivity,
) -> Result<Cached<PulledMessages>, S2sError> {
    let caller = resolve_caller(state, token).await?;
    let db_owner = DbIdentity(req.channel.identity);
    let db_channel_id = DbChannelId(req.channel.channel);
    let start = u64_to_i64(req.start);
    let limit = req.limit.min(PULL_BATCH_LIMIT);
    return s2s_tx(&state.db, "querying activity", move |db_tx| {
        let (account, local) =
            not_found(owner_channel_get(db_tx, db_owner, &db_channel_id)?, "Channel not found on this server")?;
        check_read_access(db_tx, account, db_owner, &db_channel_id, &local, caller)?;
        let rows = good_ormning::sqlite::good_query_many!(
            db,
            r#"select seq, snap_offset, receive_time, raw from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and seq >= ${i64 = start} order by seq limit ${i64 = limit as i64}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        let mut messages = Vec::with_capacity(rows.len());
        for r in rows {
            messages.push(PulledMessage {
                seq: i64_to_u64(r.seq),
                snap_offset: r.snap_offset.map(i64_to_u64),
                receive_time: r.receive_time.0,
                message: serde_json::from_str(&r.raw).context("Stored message was not valid JSON")?,
            });
        }
        let full = messages.len() as u32 == limit && limit > 0;
        return Ok(Cached {
            value: PulledMessages { messages: messages },
            policy: CachePolicy::Immutable.only_if(local.public_read && full),
        });
    }).await;
}

pub async fn handle_pull_outbox(
    state: &Arc<State>,
    token: &str,
    req: PullOutbox,
) -> Result<PulledMessages, S2sError> {
    let caller = require_caller(state, token).await?;
    let owner = req.channel.identity;
    let host = req.host;
    let db_owner = DbIdentity(owner);
    let db_host = DbIdentity(host);
    let db_channel_id = DbChannelId(req.channel.channel.clone());
    let after = req.after.map(u64_to_i64).unwrap_or(-1);
    let limit = req.limit.min(PULL_BATCH_LIMIT) as i64;
    if caller != owner {
        return Err(S2sError::Forbidden("Only the channel owner can pull a member's outbox".to_string()));
    }
    return s2s_tx(&state.db, "querying outbox for pull", move |db_tx| {
        let host_account =
            not_found(hosted_identity_account(db_tx, db_host)?, "Pull host identity is not hosted on this server")?;
        let local =
            not_found(
                local_channel_get(db_tx, host_account, db_owner, &db_channel_id)?,
                "Channel not found on this server",
            )?;
        if local.local_identity != host {
            return Err(
                S2sError::BadRequest(
                    "Pull host doesn't participate in the channel as the requested identity".to_string(),
                ),
            );
        }
        let rows = good_ormning::sqlite::good_query_many!(
            db,
            r#"select sender_unique, created, raw from outbox where account_id = ${account_id_t = &host_account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and sender = ${identity_id_t = &db_host} and sender_unique > ${i64 = after} order by sender_unique limit ${i64 = limit}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        let mut messages = Vec::with_capacity(rows.len());
        for r in rows {
            messages.push(PulledMessage {
                seq: i64_to_u64(r.sender_unique),
                snap_offset: None,
                receive_time: r.created.0,
                message: serde_json::from_str(&r.raw).context("Stored outbox message was not valid JSON")?,
            });
        }
        return Ok(PulledMessages { messages: messages });
    }).await;
}

pub async fn handle_get_snap_page(
    state: &Arc<State>,
    token: &str,
    req: GetSnapPage,
) -> Result<Cached<Option<PulledMessages>>, S2sError> {
    let caller = resolve_caller(state, token).await?;
    let db_owner = DbIdentity(req.channel.identity);
    let db_channel_id = DbChannelId(req.channel.channel);
    let page = req.page;
    return s2s_tx(&state.db, "reading snap page", move |db_tx| {
        let (account, local) =
            not_found(owner_channel_get(db_tx, db_owner, &db_channel_id)?, "Channel not found on this server")?;
        check_read_access(db_tx, account, db_owner, &db_channel_id, &local, caller)?;
        let (originals, edits) = snap_page_rows(db_tx, account, db_owner, &db_channel_id, local.page_size, page)?;
        if originals.is_empty() && page > 0 {
            return Ok(Cached::no_store(None));
        }
        let mut messages = Vec::with_capacity(originals.len() + edits.len());
        for r in originals.iter().chain(edits.iter()) {
            messages.push(r.to_pulled()?);
        }
        return Ok(Cached {
            value: Some(PulledMessages { messages: messages }),
            policy: CachePolicy::MaxAge(SNAP_PAGE_MAX_AGE_SECS).only_if(local.public_read),
        });
    }).await;
}

pub async fn handle_snap_page_containing_time(
    state: &Arc<State>,
    token: &str,
    req: GetSnapPageContainingTime,
) -> Result<Cached<Option<u64>>, S2sError> {
    let caller = resolve_caller(state, token).await?;
    let db_owner = DbIdentity(req.channel.identity);
    let db_channel_id = DbChannelId(req.channel.channel);
    let time = req.time;
    return s2s_tx(&state.db, "finding snap page for time", move |db_tx| {
        let (account, local) =
            not_found(owner_channel_get(db_tx, db_owner, &db_channel_id)?, "Channel not found on this server")?;
        check_read_access(db_tx, account, db_owner, &db_channel_id, &local, caller)?;
        let Some(offset) = snap_offset_containing_time(db_tx, account, db_owner, &db_channel_id, time)? else {
            return Ok(Cached::no_store(None));
        };
        return Ok(Cached::no_store(Some(i64_to_u64(offset) / local.page_size as u64)));
    }).await;
}

pub async fn handle_snap_lookup(
    state: &Arc<State>,
    token: &str,
    req: GetSnapLookup,
) -> Result<Cached<Option<SnapLookupRes>>, S2sError> {
    let caller = resolve_caller(state, token).await?;
    let db_owner = DbIdentity(req.channel.identity);
    let db_channel_id = DbChannelId(req.channel.channel);
    let by = req.by;
    return s2s_tx(&state.db, "looking up snap position", move |db_tx| {
        let (account, local) =
            not_found(owner_channel_get(db_tx, db_owner, &db_channel_id)?, "Channel not found on this server")?;
        check_read_access(db_tx, account, db_owner, &db_channel_id, &local, caller)?;
        let pos = match by {
            SnapLookupBy::Id(id) => snap_position_by_id(db_tx, account, db_owner, &db_channel_id, &id)?,
            SnapLookupBy::ClientId(sender, client_id) => snap_position_by_client_id(
                db_tx,
                account,
                db_owner,
                &db_channel_id,
                sender,
                &client_id.0,
            )?,
        };
        let found = pos.is_some();
        return Ok(Cached {
            value: pos.map(|p| SnapLookupRes {
                original_id: p.original_id,
                snap_offset: i64_to_u64(p.snap_offset),
                receive_time: p.receive_time,
            }),
            policy: CachePolicy::Immutable.only_if(local.public_read && found),
        });
    }).await;
}

pub async fn handle_get_message_history(
    state: &Arc<State>,
    token: &str,
    req: GetMessageHistory,
) -> Result<Cached<Option<PulledMessages>>, S2sError> {
    let caller = resolve_caller(state, token).await?;
    let db_owner = DbIdentity(req.channel.identity);
    let db_channel_id = DbChannelId(req.channel.channel);
    let id = req.id;
    return s2s_tx(&state.db, "reading message history", move |db_tx| {
        let (account, local) =
            not_found(owner_channel_get(db_tx, db_owner, &db_channel_id)?, "Channel not found on this server")?;
        check_read_access(db_tx, account, db_owner, &db_channel_id, &local, caller)?;
        let Some(rows) = message_history_rows(db_tx, account, db_owner, &db_channel_id, &id)? else {
            return Ok(Cached::no_store(None));
        };
        let mut messages = Vec::with_capacity(rows.len());
        for r in &rows {
            messages.push(r.to_pulled()?);
        }
        return Ok(Cached::no_store(Some(PulledMessages { messages: messages })));
    }).await;
}

pub enum PullOutcome {
    New,
    NoNew,
    Gone,
}

enum PullMode {
    OwnerPullsMember,
    MemberPullsOwner,
}

struct PullContext {
    local: LocalChannel,
    mode: PullMode,
    cursor: Option<u64>,
}

pub async fn pull_and_store(state: &Arc<State>, key: &PullKey) -> Result<PullOutcome, loga::Error> {
    let account = DbAccountId(key.account_id);
    let owner = key.owner;
    let source = key.source;
    let db_owner = DbIdentity(owner);
    let db_source = DbIdentity(source);
    let db_channel_id = DbChannelId(key.channel.clone());
    let qualified_channel = QualifiedChannelId {
        identity: owner,
        channel: key.channel.clone(),
    };
    let context = tx(&state.db, {
        let db_channel_id = db_channel_id.clone();
        move |db_tx| {
            let Some(local) = local_channel_get(db_tx, account, db_owner, &db_channel_id)? else {
                return Ok(None);
            };
            if local.is_owner(owner) {
                if !member_can_post(db_tx, account, db_owner, &db_channel_id, db_source)? && !local.public_post {
                    return Ok(None);
                }
                let cursor = good_ormning::sqlite::good_query_opt!(
                    db,
                    r#"select cursor_seq from channel_pull_cursor where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and source = ${identity_id_t = &db_source}"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
                return Ok(Some(PullContext {
                    cursor: cursor.map(i64_to_u64),
                    local: local,
                    mode: PullMode::OwnerPullsMember,
                }));
            } else {
                if source != owner {
                    return Ok(None);
                }
                return Ok(Some(PullContext {
                    cursor: local.latest_seq.map(i64_to_u64),
                    local: local,
                    mode: PullMode::MemberPullsOwner,
                }));
            }
        }
    }).await.context("Error reading pull context")?;
    let Some(context) = context else {
        return Ok(PullOutcome::Gone);
    };
    let local = context.local;
    let local_identity = local.local_identity;
    let mut cursor = context.cursor;
    let signer = match context.mode {
        PullMode::OwnerPullsMember => Some(load_signer(state, key.account_id, local_identity).await?),
        PullMode::MemberPullsOwner => channel_signer(state, key.account_id, &local).await?,
    };
    let only_sender = match context.mode {
        PullMode::OwnerPullsMember => Some(source),
        PullMode::MemberPullsOwner => None,
    };
    let mut latest_snap_offset = local.latest_snap_offset;
    let mut had_new = false;
    loop {
        let res = match context.mode {
            PullMode::OwnerPullsMember => state.s2s_client.pull_outbox(source, signer.as_ref().unwrap(), PullOutbox {
                channel: qualified_channel.clone(),
                host: source,
                after: cursor,
                limit: PULL_BATCH_LIMIT,
            }).await?,
            PullMode::MemberPullsOwner => state.s2s_client.get_activity(source, signer.as_ref(), GetActivity {
                channel: qualified_channel.clone(),
                start: cursor.map(|c| c + 1).unwrap_or(0),
                limit: PULL_BATCH_LIMIT,
            }).await?,
        };
        let Some(batch_max_seq) = res.messages.iter().map(|m| m.seq).max() else {
            break;
        };
        let batch_len = res.messages.len();
        let verified = verify_pulled(state, &qualified_channel, res.messages, only_sender).await?;
        let batch_snap_max = verified.iter().filter_map(|v| v.snap_offset).max().map(u64_to_i64);
        if let Some(s) = batch_snap_max {
            latest_snap_offset = Some(latest_snap_offset.map(|l| l.max(s)).unwrap_or(s));
        }
        let is_owner_mode = matches!(context.mode, PullMode::OwnerPullsMember);
        let ingested = tx(&state.db, {
            let db_channel_id = db_channel_id.clone();
            let cursor_seq = u64_to_i64(batch_max_seq);
            move |db_tx| {
                let mut ingested: Vec<(u64, String)> = vec![];
                if is_owner_mode {
                    for v in &verified {
                        if let Some(seq) =
                            ingest_message(db_tx, account, db_owner, &db_channel_id, &v.raw, &v.body)? {
                            ingested.push((seq, v.body.body.clone()));
                        }
                    }
                    good_ormning::sqlite::good_query!(
                        db,
                        r#"insert into channel_pull_cursor (account_id, identity, channel, source, cursor_seq) values (${account_id_t = &account}, ${identity_id_t = &db_owner}, ${channel_id_t = &db_channel_id}, ${identity_id_t = &db_source}, ${i64 = cursor_seq}) on conflict (account_id, identity, channel, source) do update set cursor_seq = excluded.cursor_seq"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?;
                } else {
                    for v in &verified {
                        if store_remote_message(db_tx, account, db_owner, &db_channel_id, local_identity, v, None)? {
                            ingested.push((v.seq, v.body.body.clone()));
                        }
                    }
                    good_ormning::sqlite::good_query!(
                        db,
                        r#"update channel set latest_seq = ${opt i64 = Some(cursor_seq)}, latest_snap_offset = ${opt i64 = latest_snap_offset} where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?;
                }
                return Ok(ingested);
            }
        }).await.context("Error storing pulled messages")?;
        cursor = Some(batch_max_seq);
        if let Some((seq, body)) = ingested.last().cloned() {
            had_new = true;
            publish_local_notification(state, key.account_id, qualified_channel.clone(), seq, body, true).await;
            if is_owner_mode {
                let members = tx(&state.db, {
                    let db_channel_id = db_channel_id.clone();
                    move |db_tx| {
                        return channel_members(db_tx, account, db_owner, &db_channel_id);
                    }
                }).await.context("Error listing channel members for rebroadcast")?;
                notify_remotes(
                    state,
                    key.account_id,
                    local_identity,
                    qualified_channel.clone(),
                    members.into_iter().filter(|m| *m != source).collect(),
                ).await;
            }
        }
        if (batch_len as u32) < PULL_BATCH_LIMIT {
            break;
        }
    }
    if had_new {
        return Ok(PullOutcome::New);
    } else {
        return Ok(PullOutcome::NoNew);
    }
}

pub async fn member_fetch_snap_page(
    state: &Arc<State>,
    account_id: AccountId,
    channel: &QualifiedChannelId,
    local: &LocalChannel,
    page: u64,
) -> Result<Option<Vec<Verified>>, loga::Error> {
    let signer = channel_signer(state, account_id, local).await?;
    let Some(res) = state.s2s_client.get_snap_page(channel.identity, signer.as_ref(), GetSnapPage {
        channel: channel.clone(),
        page: page,
    }).await.context("Error fetching snap page from channel owner")? else {
        return Ok(None);
    };
    let verified = verify_pulled(state, channel, res.messages, None).await?;
    store_fetched(state, account_id, channel, local, &verified).await?;
    return Ok(Some(verified));
}

pub async fn member_fetch_activity_range(
    state: &Arc<State>,
    account_id: AccountId,
    channel: &QualifiedChannelId,
    local: &LocalChannel,
    start: u64,
    end: u64,
) -> Result<Vec<Verified>, loga::Error> {
    let signer = channel_signer(state, account_id, local).await?;
    let mut next = start;
    let mut messages = vec![];
    loop {
        let limit = u32::try_from(end - next).unwrap_or(u32::MAX).min(PULL_BATCH_LIMIT);
        let res = state.s2s_client.get_activity(channel.identity, signer.as_ref(), GetActivity {
            channel: channel.clone(),
            start: next,
            limit: limit,
        }).await.context("Error fetching activity range from channel owner")?;
        let batch_len = res.messages.len();
        let Some(batch_max) = res.messages.iter().map(|m| m.seq).max() else {
            break;
        };
        messages.extend(res.messages.into_iter().filter(|m| m.seq < end));
        if batch_max + 1 >= end || (batch_len as u32) < limit {
            break;
        }
        next = batch_max + 1;
    }
    let verified = verify_pulled(state, channel, messages, None).await?;
    store_fetched(state, account_id, channel, local, &verified).await?;
    return Ok(verified);
}

pub async fn member_fetch_snap_page_containing_time(
    state: &Arc<State>,
    account_id: AccountId,
    channel: &QualifiedChannelId,
    local: &LocalChannel,
    time: Timestamp,
) -> Result<Option<u64>, loga::Error> {
    let signer = channel_signer(state, account_id, local).await?;
    return state
        .s2s_client
        .get_snap_page_containing_time(channel.identity, signer.as_ref(), GetSnapPageContainingTime {
            channel: channel.clone(),
            time: time,
        })
        .await
        .context("Error looking up snap page by time from channel owner");
}

pub async fn member_fetch_snap_lookup(
    state: &Arc<State>,
    account_id: AccountId,
    channel: &QualifiedChannelId,
    local: &LocalChannel,
    by: SnapLookupBy,
) -> Result<Option<SnapLookupRes>, loga::Error> {
    let signer = channel_signer(state, account_id, local).await?;
    return state.s2s_client.get_snap_lookup(channel.identity, signer.as_ref(), GetSnapLookup {
        channel: channel.clone(),
        by: by,
    }).await.context("Error looking up snap position from channel owner");
}

pub async fn member_fetch_message_history(
    state: &Arc<State>,
    account_id: AccountId,
    channel: &QualifiedChannelId,
    local: &LocalChannel,
    id: &MessageId,
) -> Result<Option<Vec<Verified>>, loga::Error> {
    let signer = channel_signer(state, account_id, local).await?;
    let Some(res) = state.s2s_client.get_message_history(channel.identity, signer.as_ref(), GetMessageHistory {
        channel: channel.clone(),
        id: id.clone(),
    }).await.context("Error fetching message history from channel owner")? else {
        return Ok(None);
    };
    let verified = verify_pulled(state, channel, res.messages, None).await?;
    store_fetched(state, account_id, channel, local, &verified).await?;
    return Ok(Some(verified));
}

pub fn store_channel_info(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    info: &ChannelInfoRes,
    init: bool,
) -> Result<(), loga::Error> {
    let Some(local) = local_channel_get(db_tx, account, owner, channel)? else {
        return Ok(());
    };
    let floor = info.earliest_snap_offset.map(u64_to_i64);
    let latest_snap_offset = match (local.latest_snap_offset, info.latest_snap_offset.map(u64_to_i64)) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    let latest_seq = if init && local.latest_seq.is_none() {
        info.latest_seq.map(u64_to_i64)
    } else {
        local.latest_seq
    };
    good_ormning::sqlite::good_query!(
        db,
        r#"update channel set floor_snap_offset = ${opt i64 = floor}, latest_snap_offset = ${opt i64 = latest_snap_offset}, latest_seq = ${opt i64 = latest_seq} where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and id = ${channel_id_t = channel}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    return Ok(());
}

pub async fn member_refresh_info(
    state: &Arc<State>,
    account_id: AccountId,
    channel: &QualifiedChannelId,
    local: &LocalChannel,
) -> Result<ChannelInfoRes, loga::Error> {
    let signer = channel_signer(state, account_id, local).await?;
    let info =
        state
            .s2s_client
            .get_channel_info(channel.identity, signer.as_ref(), GetChannelInfo { channel: channel.clone() })
            .await
            .context("Error fetching channel info from channel owner")?;
    let account = DbAccountId(account_id);
    let db_owner = DbIdentity(channel.identity);
    let db_channel_id = DbChannelId(channel.channel.clone());
    tx(&state.db, {
        let info = info.clone();
        move |db_tx| {
            return store_channel_info(db_tx, account, db_owner, &db_channel_id, &info, false);
        }
    }).await.context("Error storing channel info")?;
    return Ok(info);
}

async fn store_fetched(
    state: &Arc<State>,
    account_id: AccountId,
    channel: &QualifiedChannelId,
    local: &LocalChannel,
    verified: &Vec<Verified>,
) -> Result<(), loga::Error> {
    let cutoff = retention_cutoff(Timestamp::now(), local.retention_secs)?;
    let local_identity = local.local_identity;
    let rows = verified.clone();
    let account = DbAccountId(account_id);
    let db_owner = DbIdentity(channel.identity);
    let db_channel_id = DbChannelId(channel.channel.clone());
    tx(&state.db, move |db_tx| {
        for v in &rows {
            store_remote_message(db_tx, account, db_owner, &db_channel_id, local_identity, v, cutoff)?;
        }
        return Ok(());
    }).await.context("Error caching fetched messages")?;
    return Ok(());
}

struct ChannelInviteLookup {
    account_id: DbAccountId,
    owner: DbIdentity,
    can_post: bool,
    used_count: i64,
    shown_name: Option<String>,
    shown_description: Option<String>,
}

fn lookup_channel_invite(
    db_tx: &mut db::Db<Transaction<'_>>,
    channel: &DbChannelId,
    token: &str,
) -> Result<ChannelInviteLookup, S2sError> {
    let invite_row = not_found(good_ormning::sqlite::good_query_opt!(
        db,
        r#"select account_id, identity, single_use, expiry, used_count, can_post, shown_name, shown_description from channel_invite where channel = ${channel_id_t = channel} and token = ${string = token}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?, "Invitation not recognized")?;
    if let Some(exp) = invite_row.expiry {
        if exp.0 <= Timestamp::now() {
            return Err(S2sError::Forbidden("Invitation has expired".to_string()));
        }
    }
    if invite_row.single_use && invite_row.used_count > 0 {
        return Err(S2sError::Forbidden("Single-use invitation already redeemed".to_string()));
    }
    return Ok(ChannelInviteLookup {
        account_id: invite_row.account_id,
        owner: invite_row.identity,
        can_post: invite_row.can_post,
        used_count: invite_row.used_count,
        shown_name: invite_row.shown_name,
        shown_description: invite_row.shown_description,
    });
}

struct IdentityInviteLookup {
    account_id: DbAccountId,
    used_count: i64,
    name: String,
    description: String,
    shown_name: Option<String>,
    shown_description: Option<String>,
}

fn lookup_identity_invite(
    db_tx: &mut db::Db<Transaction<'_>>,
    identity: DbIdentity,
    token: &str,
) -> Result<IdentityInviteLookup, S2sError> {
    let invite_row = not_found(good_ormning::sqlite::good_query_opt!(
        db,
        r#"select account_id, single_use, expiry, used_count, name, description, shown_name, shown_description from identity_invite where identity = ${identity_id_t = &identity} and token = ${string = token}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?, "Invitation not recognized")?;
    if let Some(exp) = invite_row.expiry {
        if exp.0 <= Timestamp::now() {
            return Err(S2sError::Forbidden("Invitation has expired".to_string()));
        }
    }
    if invite_row.single_use && invite_row.used_count > 0 {
        return Err(S2sError::Forbidden("Single-use invitation already redeemed".to_string()));
    }
    let owned = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select 1 as x from identity where account_id = ${account_id_t = &invite_row.account_id} and id = ${identity_id_t = &identity} and soft_deleted_at is null"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    if owned.is_none() {
        return Err(S2sError::NotFound("Invited identity no longer exists".to_string()));
    }
    return Ok(IdentityInviteLookup {
        account_id: invite_row.account_id,
        used_count: invite_row.used_count,
        name: invite_row.name,
        description: invite_row.description,
        shown_name: invite_row.shown_name,
        shown_description: invite_row.shown_description,
    });
}

struct OwnerChannelMeta {
    name: String,
    description: String,
    shown_name: Option<String>,
    shown_description: Option<String>,
    portrait: Option<String>,
}

fn owner_channel_meta(
    db_tx: &mut db::Db<Transaction<'_>>,
    account_id: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
) -> Result<OwnerChannelMeta, S2sError> {
    let row = not_found(good_ormning::sqlite::good_query_opt!(
        db,
        r#"select name, description, shown_name, shown_description, portrait from channel where account_id = ${account_id_t = &account_id} and identity = ${identity_id_t = &owner} and id = ${channel_id_t = channel} and deleted is null"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?, "Channel no longer exists")?;
    return Ok(OwnerChannelMeta {
        name: row.name,
        description: row.description,
        shown_name: row.shown_name,
        shown_description: row.shown_description,
        portrait: row.portrait,
    });
}

struct OwnerIdentityMeta {
    name: String,
    description: String,
    shown_name: Option<String>,
    shown_description: Option<String>,
    portrait: Option<String>,
}

fn owner_identity_meta(
    db_tx: &mut db::Db<Transaction<'_>>,
    account_id: DbAccountId,
    identity: DbIdentity,
) -> Result<OwnerIdentityMeta, S2sError> {
    let row = not_found(good_ormning::sqlite::good_query_opt!(
        db,
        r#"select name, description, shown_name, shown_description, portrait from identity where account_id = ${account_id_t = &account_id} and id = ${identity_id_t = &identity} and soft_deleted_at is null"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?, "Invited identity no longer exists")?;
    return Ok(OwnerIdentityMeta {
        name: row.name,
        description: row.description,
        shown_name: row.shown_name,
        shown_description: row.shown_description,
        portrait: row.portrait,
    });
}

pub async fn handle_get_file(
    state: &Arc<State>,
    token: &str,
    req: GetFile,
) -> Result<(String, Vec<u8>), S2sError> {
    let hash = req.hash.clone();
    let account = match req.auth {
        FileAuth::IdentityInvite { identity, code } => {
            let db_identity = DbIdentity(identity);
            s2s_tx(&state.db, "checking invited identity file access", move |db_tx| {
                let invite = lookup_identity_invite(db_tx, db_identity, &code)?;
                let meta = owner_identity_meta(db_tx, invite.account_id, db_identity)?;
                if meta.portrait.as_deref() != Some(hash.0.as_str()) {
                    return Err(S2sError::Forbidden("File is not part of this invitation".to_string()));
                }
                return Ok(invite.account_id);
            }).await?
        },
        FileAuth::ChannelInvite { channel, code } => {
            let db_owner = DbIdentity(channel.identity);
            let db_channel_id = DbChannelId(channel.channel);
            s2s_tx(&state.db, "checking invited channel file access", move |db_tx| {
                let invite = lookup_channel_invite(db_tx, &db_channel_id, &code)?;
                if invite.owner.0 != channel.identity {
                    return Err(S2sError::NotFound("Invitation not recognized".to_string()));
                }
                let meta = owner_channel_meta(db_tx, invite.account_id, db_owner, &db_channel_id)?;
                if meta.portrait.as_deref() != Some(hash.0.as_str()) {
                    return Err(S2sError::Forbidden("File is not part of this invitation".to_string()));
                }
                return Ok(invite.account_id);
            }).await?
        },
        FileAuth::Caller => {
            let caller = resolve_caller(state, token).await?;
            s2s_tx(&state.db, "checking file access", move |db_tx| {
                let identities = good_ormning::sqlite::good_query_many!(
                    db,
                    r#"select account_id, id from identity where portrait = ${string = hash.0.as_str()} and soft_deleted_at is null"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
                if let Some(caller) = caller {
                    let db_caller = DbIdentity(caller);
                    for i in identities {
                        let identity_owns = good_ormning::sqlite::good_query_opt!(
                            db,
                            r#"select 1 as x from channel_member where account_id = ${account_id_t = &i.account_id} and identity = ${identity_id_t = &i.id} and member = ${identity_id_t = &db_caller}"#;
                            db_tx
                        ).map_err(|e| loga::err(e.0))?;
                        if identity_owns.is_some() {
                            return Ok(i.account_id);
                        }
                        let caller_owns = good_ormning::sqlite::good_query_opt!(
                            db,
                            r#"select 1 as x from channel where account_id = ${account_id_t = &i.account_id} and identity = ${identity_id_t = &db_caller} and local_identity = ${identity_id_t = &i.id} and deleted is null"#;
                            db_tx
                        ).map_err(|e| loga::err(e.0))?;
                        if caller_owns.is_some() {
                            return Ok(i.account_id);
                        }
                    }
                }
                let channels = good_ormning::sqlite::good_query_many!(
                    db,
                    r#"select account_id, identity, id, local_identity from channel where portrait = ${string = hash.0.as_str()} and deleted is null"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
                for c in channels {
                    if c.identity.0 != c.local_identity.0 {
                        continue;
                    }
                    let Some(local) = local_channel_get(db_tx, c.account_id, c.identity, &c.id)? else {
                        continue;
                    };
                    if check_read_access(db_tx, c.account_id, c.identity, &c.id, &local, caller).is_ok() {
                        return Ok(c.account_id);
                    }
                }
                if caller.is_some() {
                    return Err(S2sError::Forbidden("Caller has no access to this file".to_string()));
                } else {
                    return Err(S2sError::Unauthorized("File requires authentication".to_string()));
                }
            }).await?
        },
    };
    return Ok(not_found(crate::subsystems::files::read_file(state, account, &req.hash).await?, "File not found")?);
}

pub async fn handle_join_preview(
    state: &Arc<State>,
    token: &str,
    req: GetJoinPreview,
) -> Result<Cached<JoinPreviewRes>, S2sError> {
    let caller = resolve_caller(state, token).await?;
    return s2s_tx(&state.db, "reading join preview", move |db_tx| {
        match req.target {
            JoinPreviewTarget::IdentityInvite { identity, code } => {
                let db_identity = DbIdentity(identity);
                let invite = lookup_identity_invite(db_tx, db_identity, &code)?;
                let meta = owner_identity_meta(db_tx, invite.account_id, db_identity)?;
                return Ok(Cached::no_store(JoinPreviewRes {
                    name: invite.shown_name.or(meta.shown_name).unwrap_or(meta.name),
                    description: invite.shown_description.or(meta.shown_description).unwrap_or(meta.description),
                    portrait: crate::subsystems::files::file_ref(db_tx, invite.account_id, meta.portrait)?,
                }));
            },
            JoinPreviewTarget::ChannelInvite { channel, code } => {
                let db_owner = DbIdentity(channel.identity);
                let db_channel_id = DbChannelId(channel.channel);
                let invite = lookup_channel_invite(db_tx, &db_channel_id, &code)?;
                if invite.owner.0 != channel.identity {
                    return Err(S2sError::NotFound("Invitation not recognized".to_string()));
                }
                let meta = owner_channel_meta(db_tx, invite.account_id, db_owner, &db_channel_id)?;
                return Ok(Cached::no_store(JoinPreviewRes {
                    name: invite.shown_name.or(meta.shown_name).unwrap_or(meta.name),
                    description: invite.shown_description.or(meta.shown_description).unwrap_or(meta.description),
                    portrait: crate::subsystems::files::file_ref(db_tx, invite.account_id, meta.portrait)?,
                }));
            },
            JoinPreviewTarget::Channel(channel) => {
                let db_owner = DbIdentity(channel.identity);
                let db_channel_id = DbChannelId(channel.channel);
                let (account, local) =
                    not_found(
                        owner_channel_get(db_tx, db_owner, &db_channel_id)?,
                        "Channel not found on this server",
                    )?;
                check_read_access(db_tx, account, db_owner, &db_channel_id, &local, caller)?;
                let meta = owner_channel_meta(db_tx, account, db_owner, &db_channel_id)?;
                return Ok(Cached::no_store(JoinPreviewRes {
                    name: meta.shown_name.unwrap_or(meta.name),
                    description: meta.shown_description.unwrap_or(meta.description),
                    portrait: crate::subsystems::files::file_ref(db_tx, account, meta.portrait)?,
                }));
            },
        }
    }).await;
}

pub async fn handle_join(
    state: &Arc<State>,
    token: &str,
    req: crate::interface::s2s::Join,
) -> Result<crate::interface::s2s::JoinRes, S2sError> {
    use crate::interface::s2s::{
        ChannelOrIdentity,
        InvitationToken,
    };

    let caller = require_caller(state, token).await?;
    match (req.target, req.invitation) {
        (ChannelOrIdentity::Channel(channel_id), InvitationToken::Channel(invite)) => {
            let db_channel_id = DbChannelId(channel_id.clone());
            let db_caller = DbIdentity(caller);
            let invite_str = invite.0.clone();
            let (account_id, owner, info) = s2s_tx(&state.db, "redeeming channel invitation", move |db_tx| {
                let invite_row = lookup_channel_invite(db_tx, &db_channel_id, &invite_str)?;
                let account_id = invite_row.account_id;
                let owner = invite_row.owner;
                let local =
                    not_found(
                        local_channel_get(db_tx, account_id, owner, &db_channel_id)?,
                        "Invited channel no longer exists",
                    )?;
                if !local.is_owner(owner.0) {
                    return Err(
                        S2sError::NotFound("Invited channel isn't owned by the inviting identity".to_string()),
                    );
                }
                if caller == owner.0 {
                    return Err(S2sError::BadRequest("Channel owner can't join their own channel".to_string()));
                }
                good_ormning::sqlite::good_query!(
                    db,
                    r#"insert into channel_member (account_id, identity, channel, member, confirmed, can_post, memo_short, memo_long) values (${account_id_t = &account_id}, ${identity_id_t = &owner}, ${channel_id_t = &db_channel_id}, ${identity_id_t = &db_caller}, ${bool = true}, ${bool = invite_row.can_post}, ${string = ""}, ${string = ""}) on conflict (account_id, identity, channel, member) do update set confirmed = true"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
                let new_count = invite_row.used_count + 1;
                good_ormning::sqlite::good_query!(
                    db,
                    r#"update channel_invite set used_count = ${i64 = new_count} where channel = ${channel_id_t = &db_channel_id} and token = ${string = invite_str.as_str()}"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
                let info = channel_info_res(db_tx, account_id, owner, &db_channel_id, &local)?;
                return Ok((account_id.0, owner.0, info));
            }).await?;
            state.pull_scheduler.add_or_reset(PullKey {
                account_id: account_id,
                owner: owner,
                channel: channel_id.clone(),
                source: caller,
            }).await;
            return Ok(crate::interface::s2s::JoinRes {
                channel: channel_id,
                info: info,
            });
        },
        (ChannelOrIdentity::Identity(identity), InvitationToken::Identity(invite)) => {
            let db_identity = DbIdentity(identity);
            let db_caller = DbIdentity(caller);
            let invite_str = invite.0.clone();
            let dm_channel_id = crate::subsystems::c2s::random_channel_id();
            let db_dm_channel_id = DbChannelId(dm_channel_id.clone());
            let page_size = state.default_page_size;
            let page_size_i64 = page_size as i64;
            if caller == identity {
                return Err(S2sError::BadRequest("Can't redeem your own identity invitation".to_string()));
            }
            let account_id = s2s_tx(&state.db, "redeeming identity invitation", move |db_tx| {
                let invite_row = lookup_identity_invite(db_tx, db_identity, &invite_str)?;
                let account_id = invite_row.account_id;
                good_ormning::sqlite::good_query!(
                    db,
                    r#"insert into contact (account_id, id, name, description) values (${account_id_t = &account_id}, ${identity_id_t = &db_caller}, ${string = ""}, ${string = ""}) on conflict do nothing"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
                let dm_idem = format!("dm:{}:{}", caller, dm_channel_id.0);
                let new_count = invite_row.used_count + 1;
                let dm_name = format!("{} - {}", invite_row.name, new_count);
                crate::subsystems::c2s::insert_channel_row(
                    db_tx,
                    account_id,
                    db_identity,
                    &db_dm_channel_id,
                    db_identity,
                    &dm_idem,
                    None,
                    &dm_name,
                    &invite_row.description,
                    None,
                    None,
                    None,
                    page_size_i64,
                    true,
                )?;
                good_ormning::sqlite::good_query!(
                    db,
                    r#"insert into channel_member (account_id, identity, channel, member, confirmed, can_post, memo_short, memo_long) values (${account_id_t = &account_id}, ${identity_id_t = &db_identity}, ${channel_id_t = &db_dm_channel_id}, ${identity_id_t = &db_caller}, ${bool = true}, ${bool = true}, ${string = ""}, ${string = ""})"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
                good_ormning::sqlite::good_query!(
                    db,
                    r#"update identity_invite set used_count = ${i64 = new_count} where identity = ${identity_id_t = &db_identity} and token = ${string = invite_str.as_str()}"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
                return Ok(account_id.0);
            }).await?;
            state.pull_scheduler.add_or_reset(PullKey {
                account_id: account_id,
                owner: identity,
                channel: dm_channel_id.clone(),
                source: caller,
            }).await;
            return Ok(crate::interface::s2s::JoinRes {
                channel: dm_channel_id,
                info: ChannelInfoRes {
                    page_size: u32::try_from(page_size).unwrap_or(u32::MAX),
                    latest_seq: None,
                    latest_snap_offset: None,
                    earliest_snap_offset: None,
                },
            });
        },
        _ => return Err(S2sError::BadRequest("Target/invitation type mismatch".to_string())),
    }
}

struct BackfillCandidate {
    account_id: AccountId,
    channel: QualifiedChannelId,
}

async fn mark_backfill_done(state: &Arc<State>, c: &BackfillCandidate) -> Result<(), loga::Error> {
    let account = DbAccountId(c.account_id);
    let db_owner = DbIdentity(c.channel.identity);
    let db_channel_id = DbChannelId(c.channel.channel.clone());
    tx(&state.db, move |db_tx| {
        good_ormning::sqlite::good_query!(
            db,
            r#"update channel set backfill_done = ${bool = true} where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        return Ok(());
    }).await.context("Error marking backfill done")?;
    return Ok(());
}

pub async fn run_backfill(state: Arc<State>) -> Result<(), loga::Error> {
    let mut tick: usize = 0;
    loop {
        tokio::time::sleep(BACKFILL_INTERVAL).await;
        let candidates = match tx(&state.db, move |db_tx| {
            return Ok(good_ormning::sqlite::good_query_many!(
                db,
                r#"select account_id, identity, id from channel where deleted is null and backfill_done = ${bool = false}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?.into_iter().map(|r| BackfillCandidate {
                account_id: r.account_id.0,
                channel: QualifiedChannelId {
                    identity: r.identity.0,
                    channel: r.id.0,
                },
            }).collect::<Vec<_>>());
        }).await {
            Ok(c) => c,
            Err(e) => {
                state.log.log_err(loga::WARN, e.context("Error listing backfill candidates"));
                continue;
            },
        };
        if candidates.is_empty() {
            continue;
        }
        let c = &candidates[tick % candidates.len()];
        tick = tick.wrapping_add(1);
        if let Err(e) = async {
            flowcontrol::ta_return!((), loga::Error);
            let state = &state;
            let account = DbAccountId(c.account_id);
            let db_owner = DbIdentity(c.channel.identity);
            let db_channel_id = DbChannelId(c.channel.channel.clone());
            let now = Timestamp::now();
            let Some((local, lowest)) = tx(&state.db, {
                let db_channel_id = db_channel_id.clone();
                move |db_tx| {
                    let Some(local) = local_channel_get(db_tx, account, db_owner, &db_channel_id)? else {
                        return Ok(None);
                    };
                    let lowest = good_ormning::sqlite::good_query_opt!(
                        db,
                        r#"select snap_offset, receive_time from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and snap_offset is not null order by snap_offset asc limit ${i64 = 1}"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?;
                    return Ok(Some((local, lowest)));
                }
            }).await.context("Error reading backfill state")? else {
                return Ok(());
            };
            let cutoff = retention_cutoff(now, local.retention_secs)?;
            let page_size = local.page_size as u64;
            let target_page = match lowest {
                Some(row) => {
                    let offset = i64_to_u64(row.snap_offset.context("Lowest snap row has no snap offset")?);
                    if let Some(cutoff) = cutoff {
                        if row.receive_time.0 < cutoff {
                            mark_backfill_done(state, c).await?;
                            return Ok(());
                        }
                    }
                    if let Some(floor) = local.floor_snap_offset {
                        if offset <= i64_to_u64(floor) {
                            mark_backfill_done(state, c).await?;
                            return Ok(());
                        }
                    }
                    let page = offset / page_size;
                    if offset % page_size == 0 {
                        if page == 0 {
                            mark_backfill_done(state, c).await?;
                            return Ok(());
                        }
                        page - 1
                    } else {
                        page
                    }
                },
                None => {
                    let info = member_refresh_info(state, c.account_id, &c.channel, &local).await?;
                    let Some(latest) = info.latest_snap_offset else {
                        return Ok(());
                    };
                    latest / page_size
                },
            };
            let Some(fetched) =
                member_fetch_snap_page(state, c.account_id, &c.channel, &local, target_page).await? else {
                    mark_backfill_done(state, c).await?;
                    return Ok(());
                };
            let originals: Vec<&Verified> = fetched.iter().filter(|v| v.snap_offset.is_some()).collect();
            if target_page == 0 || originals.is_empty() {
                mark_backfill_done(state, c).await?;
                return Ok(());
            }
            if let Some(cutoff) = cutoff {
                if originals.iter().any(|v| v.receive_time < cutoff) {
                    mark_backfill_done(state, c).await?;
                }
            }
            return Ok(());
        }.await {
            state
                .log
                .log_err(
                    loga::WARN,
                    e.context_with(
                        "Error backfilling channel",
                        ea!(owner = c.channel.identity, channel = c.channel.channel.0),
                    ),
                );
        }
    }
}

pub async fn run_retention_cleanup(state: Arc<State>) -> Result<(), loga::Error> {
    loop {
        let now = Timestamp::now();
        if let Err(e) = tx(&state.db, move |db_tx| {
            let channels = good_ormning::sqlite::good_query_many!(
                db,
                r#"select account_id, identity, id, retention_secs from channel where deleted is null and retention_secs is not null"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
            for c in channels {
                let Some(cutoff) = retention_cutoff(now, c.retention_secs)? else {
                    continue;
                };
                good_ormning::sqlite::good_query!(
                    db,
                    r#"delete from message where account_id = ${account_id_t = &c.account_id} and identity = ${identity_id_t = &c.identity} and channel = ${channel_id_t = &c.id} and receive_time < ${time_ms_t = &crate::interface::db::DbTimeMs(cutoff)}"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
            }
            return Ok(());
        }).await {
            state.log.log_err(loga::WARN, e.context("Error running retention cleanup"));
        }
        tokio::time::sleep(RETENTION_CLEANUP_INTERVAL).await;
    }
}

pub fn latest_activity_map(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
) -> Result<HashMap<QualifiedChannelId, u64>, loga::Error> {
    let rows = good_ormning::sqlite::good_query_many!(
        db,
        r#"select identity, id, latest_seq from channel where account_id = ${account_id_t = &account} and deleted is null and latest_seq is not null"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    let mut out = HashMap::new();
    for r in rows {
        let Some(seq) = r.latest_seq else {
            continue;
        };
        out.insert(QualifiedChannelId {
            identity: r.identity.0,
            channel: r.id.0,
        }, i64_to_u64(seq));
    }
    return Ok(out);
}
