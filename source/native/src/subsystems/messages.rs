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
                GetChannelInfo,
                PulledMessage,
                SnapLookupBy,
                SnapLookupRes,
            },
        },
        subsystems::{
            pull_scheduler::PullKey,
            s2s_client::Signer,
            s2s_error::{
                not_found,
                S2sError,
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
            c2s::{
                ActivityOffset,
                ChannelRetention,
            },
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
        sync::{
            Arc,
            Mutex,
        },
        time::Duration,
    },
};

pub const PULL_BATCH_LIMIT: u32 = 100;

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

pub const SECS_PER_DAY: i64 = 24 * 60 * 60;

pub fn retention_cutoff(now: Timestamp, retention: ChannelRetention) -> Result<Option<Timestamp>, loga::Error> {
    let ChannelRetention::Days(days) = retention else {
        return Ok(None);
    };
    let secs = retention_days_to_secs(days)?;
    return Ok(
        Some(
            now
                .checked_sub(Duration::from_secs(secs as u64))
                .context_with("Retention cutoff out of range", ea!(retention_secs = secs))?,
        ),
    );
}

pub fn retention_days_to_secs(days: u64) -> Result<i64, loga::Error> {
    return Ok(
        i64::try_from(days)
            .ok()
            .and_then(|d| d.checked_mul(SECS_PER_DAY))
            .context_with("Retention days out of range", ea!(days = days))?,
    );
}

pub const RETENTION_MODE_NOTHING: &str = "nothing";
pub const RETENTION_MODE_VIEWED: &str = "viewed";
pub const RETENTION_MODE_ALL: &str = "all";
pub const RETENTION_MODE_DAYS: &str = "days";

pub fn retention_from_db(mode: &str, secs: Option<i64>) -> ChannelRetention {
    match mode {
        RETENTION_MODE_NOTHING => return ChannelRetention::Nothing,
        RETENTION_MODE_VIEWED => return ChannelRetention::Viewed,
        RETENTION_MODE_ALL => return ChannelRetention::All,
        RETENTION_MODE_DAYS => return ChannelRetention::Days(
            (secs.expect("Days retention mode stored without retention_secs") / SECS_PER_DAY) as u64,
        ),
        _ => panic!("Unknown retention mode stored in db: {}", mode),
    }
}

#[derive(Clone)]
pub struct LocalChannel {
    pub local_identity: Identity,
    pub page_size: usize,
    pub authenticated: bool,
    pub public_read: bool,
    pub public_post: bool,
    pub retention: ChannelRetention,
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
        r#"select local_identity, page_size, authenticated, public_read, public_post, retention_secs, retention_mode, latest_seq, latest_snap_offset, floor_snap_offset, backfill_done from channel where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and id = ${channel_id_t = channel} and deleted is null"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    return Ok(row.map(|r| LocalChannel {
        local_identity: r.local_identity.0,
        page_size: r.page_size.max(1) as usize,
        authenticated: r.authenticated,
        public_read: r.public_read,
        public_post: r.public_post,
        retention: retention_from_db(&r.retention_mode, r.retention_secs),
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
    if caller == owner.0 || {
        (|| -> Result<bool, loga::Error> {
            let (db_tx, account, owner, channel, member):
                (&mut db::Db<Transaction<'_>>, DbAccountId, DbIdentity, &DbChannelId, DbIdentity) =
                (db_tx, account, owner, channel, DbIdentity(caller));
            return Ok(good_ormning::sqlite::good_query_opt!(
                db,
                r#"select 1 as x from channel_member where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and member = ${identity_id_t = &member}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?.is_some());
        })()
    }? {
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
    return Ok(db_tx.0.changes() > 0);
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
            if let Err(e) = {
                use {
                    crate::{
                        db,
                        dbutil::tx,
                        State,
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
                    },
                    loga::{
                        ea,
                        ErrContext,
                        ResultContext,
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
                use crate::subsystems::push::*;

                let ret: Result<(), loga::Error> = async {
                    let (state, account_id, payload): (&Arc<State>, AccountId, &[u8]) =
                        (&state, account_id, &payload);
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
                                        loga::err(
                                            e.to_string(),
                                        ).context("Skipping malformed push subscription record"),
                                    );
                                continue;
                            },
                        };
                        let endpoint = info.endpoint.clone();
                        let res = async {
                            flowcontrol::ta_return!((), PushSendError);
                            let vapid_sig =
                                VapidSignatureBuilder::from_base64_no_sub(&vapid.private_b64url)
                                    .map_err(
                                        |e| PushSendError::Other(
                                            loga::err(e.to_string()).context("VAPID key parse failed"),
                                        ),
                                    )?
                                    .add_sub_info(&info)
                                    .build()
                                    .map_err(
                                        |e| PushSendError::Other(
                                            loga::err(e.to_string()).context("VAPID signature build failed"),
                                        ),
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
                                headers.insert(
                                    "Content-Encoding".to_string(),
                                    p.content_encoding.to_str().to_string(),
                                );
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
                                    |e| PushSendError::Other(
                                        loga::err(e.to_string()).context("Invalid push endpoint URL"),
                                    ),
                                )?;
                            let authority =
                                endpoint_uri
                                    .authority()
                                    .ok_or_else(
                                        || PushSendError::Other(loga::err("Push endpoint URL has no authority")),
                                    )?
                                    .to_string();
                            let mut conn: htreq::Conn =
                                htreq::connect(PUSH_LIMITS, &endpoint_uri)
                                    .await
                                    .map_err(
                                        |e| PushSendError::Other(e.context("Failed to connect to push endpoint")),
                                    )?;
                            let mut req =
                                Request::builder()
                                    .method(Method::POST)
                                    .uri(endpoint_uri.clone())
                                    .header(HOST, authority);
                            for (k, v) in headers {
                                req = req.header(k, v);
                            }
                            let req =
                                req
                                    .body(http_body_util::Full::new(hyper::body::Bytes::from(body)))
                                    .map_err(
                                        |e| PushSendError::Other(
                                            loga::err(e.to_string()).context("Error building push request"),
                                        ),
                                    )?;
                            let (status, _, continue_) =
                                htreq::send(&state.log, PUSH_LIMITS, &mut conn, req)
                                    .await
                                    .map_err(|e| PushSendError::Other(e.context("Push endpoint POST failed")))?;
                            if status == StatusCode::NOT_FOUND || status == StatusCode::GONE {
                                return Err(PushSendError::Gone);
                            }
                            if !status.is_success() {
                                let err =
                                    loga::err_with("Push endpoint returned error status", ea!(status = status));
                                match htreq::receive(PUSH_LIMITS, continue_).await {
                                    Ok(body) => {
                                        return Err(
                                            PushSendError::Other(
                                                err.context_with(
                                                    "Error response body",
                                                    ea!(body = String::from_utf8_lossy(&body)),
                                                ),
                                            ),
                                        );
                                    },
                                    Err(e) => {
                                        return Err(
                                            PushSendError::Other(
                                                err.also(e.context("Error reading error response body")),
                                            ),
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
                                state
                                    .log
                                    .log_err(
                                        loga::WARN,
                                        e.context_with("Push delivery failed", ea!(endpoint = endpoint)),
                                    );
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
                }.await;
                ret
            } {
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
            if let Err(e) = {
                use {
                    crate::interface::s2s::{
                        s2sv1,
                        s2sv1t::NotifyTimestamp,
                        Notify,
                    },
                    spaghettinuum::{
                        byteszb32::BytesZb32,
                        interface::identity::Identity,
                        jsonsig::JsonSignature,
                        sigdomain::SigDomained,
                    },
                    std::collections::HashMap,
                };
                use crate::subsystems::s2s_client::*;

                let ret: Result<(), loga::Error> = async {
                    let (s2s_client, target, signer, owner, channel):
                        (&S2sClient, Identity, &Signer, Identity, shared::interface::shared::ChannelId) =
                        (&(client), recipient, &signer, channel.identity, channel.channel);
                    let ts = NotifyTimestamp {
                        owner: owner,
                        channel: channel,
                        timestamp: jiff::Timestamp::now(),
                    };
                    let message = serde_json::to_string(&ts).unwrap();
                    let (_root, _chain, sig) =
                        signer.lock().unwrap().sign(NotifyTimestamp::DOMAIN, message.as_bytes())?;
                    let signed = JsonSignature {
                        message: message,
                        signature: BytesZb32(sig),
                        _p: Default::default(),
                    };
                    return s2s_client.post(target, &HashMap::new(), &s2sv1::Req::Notify(Notify {
                        signer: signer_identity(signer)?,
                        signed: signed,
                    })).await;
                }.await;
                ret
            } {
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

pub async fn resolve_caller(state: &Arc<State>, token: &str) -> Result<Option<Identity>, S2sError> {
    if token.is_empty() {
        return Ok(None);
    }
    match {
        use spaghettinuum::interface::identity::Identity;
        use crate::subsystems::s2s::*;

        let ret: Option<Identity> = async {
            let (s2s_state, token): (&S2sState, &str) = (&(state.s2s_state), token);
            return s2s_state.tokens.get(token).await;
        }.await;
        ret
    } {
        Some(identity) => return Ok(Some(identity)),
        None => return Err(S2sError::Unauthorized("Request token is invalid or expired".to_string())),
    }
}

pub async fn require_caller(state: &Arc<State>, token: &str) -> Result<Identity, S2sError> {
    match resolve_caller(state, token).await? {
        Some(identity) => return Ok(identity),
        None => return Err(S2sError::Unauthorized("Request requires authentication".to_string())),
    }
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

pub enum PullOutcome {
    New,
    NoNew,
    Gone,
}

pub enum PullMode {
    OwnerPullsMember,
    MemberPullsOwner,
}

pub struct PullContext {
    pub local: LocalChannel,
    pub mode: PullMode,
    pub cursor: Option<u64>,
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
    let retention = local.retention;
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
            PullMode::OwnerPullsMember => {
                use {
                    crate::interface::s2s::{
                        s2sv1,
                        PullOutbox,
                        PulledMessages,
                    },
                    spaghettinuum::interface::identity::Identity,
                };
                use crate::subsystems::s2s_client::*;

                let ret: Result<PulledMessages, loga::Error> = async {
                    let (s2s_client, target, signer, req): (&S2sClient, Identity, &Signer, PullOutbox) =
                        (&(state.s2s_client), source, signer.as_ref().unwrap(), PullOutbox {
                            channel: qualified_channel.clone(),
                            host: source,
                            after: cursor,
                            limit: PULL_BATCH_LIMIT,
                        });
                    return s2s_client.post_authed(target, signer, &s2sv1::Req::PullOutbox(req)).await;
                }.await;
                ret
            }?,
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
        let is_owner_mode = matches!(context.mode, PullMode::OwnerPullsMember);
        let mut verified = verify_pulled(state, &qualified_channel, res.messages, only_sender).await?;
        if is_owner_mode {
            verified.retain(|v| {
                let own = match &v.body.rel {
                    MessageRel::EditOf(target) => target.identity == v.body.id.identity,
                    MessageRel::None | MessageRel::ReplyTo(_) => true,
                };
                if own {
                    return true;
                }
                state
                    .log
                    .log_with(
                        loga::WARN,
                        "Refusing pulled edit: target has a different sender",
                        ea!(sender = v.body.id.identity),
                    );
                return false;
            });
        }
        let batch_snap_max = verified.iter().filter_map(|v| v.snap_offset).max().map(u64_to_i64);
        if let Some(s) = batch_snap_max {
            latest_snap_offset = Some(latest_snap_offset.map(|l| l.max(s)).unwrap_or(s));
        }
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
                        let new = match retention {
                            ChannelRetention::Nothing => true,
                            ChannelRetention::Viewed | ChannelRetention::All | ChannelRetention::Days(_) => {
                                store_remote_message(db_tx, account, db_owner, &db_channel_id, v, None)?
                            },
                        };
                        if new {
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
    let Some(res) = {
        use {
            crate::interface::s2s::{
                GetSnapPage,
                PulledMessages,
            },
            spaghettinuum::interface::identity::Identity,
        };
        use crate::subsystems::s2s_client::*;

        let ret: Result<Option<PulledMessages>, loga::Error> = async {
            let (s2s_client, target, signer, req): (&S2sClient, Identity, Option<&Signer>, GetSnapPage) =
                (&(state.s2s_client), channel.identity, signer.as_ref(), GetSnapPage {
                    channel: channel.clone(),
                    page: page,
                });
            return s2s_client.get_opt_authed(target, signer, &req).await;
        }.await;
        ret
    }.context("Error fetching snap page from channel owner")? else {
        return Ok(None);
    };
    let verified = verify_pulled(state, channel, res.messages, None).await?;
    store_fetched(state, account_id, channel, local, &verified).await?;
    return Ok(Some(verified));
}

pub async fn member_fetch_snap_lookup(
    state: &Arc<State>,
    account_id: AccountId,
    channel: &QualifiedChannelId,
    local: &LocalChannel,
    by: SnapLookupBy,
) -> Result<Option<SnapLookupRes>, loga::Error> {
    let signer = channel_signer(state, account_id, local).await?;
    return {
        use {
            crate::interface::s2s::{
                GetSnapLookup,
                SnapLookupRes,
            },
            spaghettinuum::interface::identity::Identity,
        };
        use crate::subsystems::s2s_client::*;

        let ret: Result<Option<SnapLookupRes>, loga::Error> = async {
            let (s2s_client, target, signer, req): (&S2sClient, Identity, Option<&Signer>, GetSnapLookup) =
                (&(state.s2s_client), channel.identity, signer.as_ref(), GetSnapLookup {
                    channel: channel.clone(),
                    by: by,
                });
            return s2s_client.get_opt_authed(target, signer, &req).await;
        }.await;
        ret
    }.context("Error looking up snap position from channel owner");
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

pub async fn store_fetched(
    state: &Arc<State>,
    account_id: AccountId,
    channel: &QualifiedChannelId,
    local: &LocalChannel,
    verified: &Vec<Verified>,
) -> Result<(), loga::Error> {
    match local.retention {
        ChannelRetention::Nothing => return Ok(()),
        ChannelRetention::Viewed | ChannelRetention::All | ChannelRetention::Days(_) => { },
    }
    let cutoff = retention_cutoff(Timestamp::now(), local.retention)?;
    let rows = verified.clone();
    let account = DbAccountId(account_id);
    let db_owner = DbIdentity(channel.identity);
    let db_channel_id = DbChannelId(channel.channel.clone());
    tx(&state.db, move |db_tx| {
        for v in &rows {
            store_remote_message(db_tx, account, db_owner, &db_channel_id, v, cutoff)?;
        }
        return Ok(());
    }).await.context("Error caching fetched messages")?;
    return Ok(());
}

pub struct ChannelInviteLookup {
    pub account_id: DbAccountId,
    pub owner: DbIdentity,
    pub can_post: bool,
    pub used_count: i64,
    pub shown_name: Option<String>,
    pub shown_description: Option<String>,
}

pub fn lookup_channel_invite(
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

pub struct IdentityInviteLookup {
    pub account_id: DbAccountId,
    pub used_count: i64,
    pub name: String,
    pub description: String,
    pub shown_name: Option<String>,
    pub shown_description: Option<String>,
}

pub fn lookup_identity_invite(
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

pub struct OwnerChannelMeta {
    pub name: String,
    pub description: String,
    pub shown_name: Option<String>,
    pub shown_description: Option<String>,
    pub portrait: Option<String>,
}

pub fn owner_channel_meta(
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

pub struct OwnerIdentityMeta {
    pub name: String,
    pub description: String,
    pub shown_name: Option<String>,
    pub shown_description: Option<String>,
    pub portrait: Option<String>,
}

pub fn owner_identity_meta(
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

pub struct BackfillCandidate {
    pub account_id: AccountId,
    pub channel: QualifiedChannelId,
}

pub async fn mark_backfill_done(state: &Arc<State>, c: &BackfillCandidate) -> Result<(), loga::Error> {
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
