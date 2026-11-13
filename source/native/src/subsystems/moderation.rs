use {
    crate::{
        db,
        dbutil::tx,
        interface::{
            db::{
                DbAccountId,
                DbChannelId,
                DbIdentity,
                DbTimeMs,
            },
            s2s::{
                s2sv1,
                Moderate,
                ModerateReq,
                ModerateRes,
            },
        },
        subsystems::{
            c2s::random_token,
            messages::{
                channel_members,
                i64_to_u64,
                ingest_message,
                local_channel_get,
                notify_remotes,
                parse_blocks,
                publish_local_notification,
                u64_to_i64,
                LocalChannel,
                MESSAGE_FILE_SOURCE_MESSAGE,
                REL_EDIT_OF,
                REL_REPLY_TO,
            },
            pull_scheduler::PullKey,
            s2s_error::{
                not_found,
                s2s_tx,
                S2sError,
            },
        },
        State,
    },
    loga::ResultContext,
    rusqlite::Transaction,
    shared::interface::{
        shared::{
            ChannelInviteId,
            ChannelInviteToken,
            Level,
            Message,
            MessageBlock,
            MessageBody,
            MessageId,
            MessageRel,
            ModerationState,
            QualifiedChannelId,
            QualifiedChannelInviteToken,
        },
        wire::{
            c2s::{
                ChannelInviteRes,
                ChannelMemberRes,
                ModerationCursor,
                ModerationEntry,
                MODERATION_PAGE_LIMIT,
            },
            s2c::{
                ModerationChange,
                WsEvent,
            },
        },
    },
    spaghettinuum::interface::identity::Identity,
    std::sync::Arc,
};

pub const LEVEL_READ_ONLY: &str = "read_only";
pub const LEVEL_PROBATIONAL: &str = "probational";
pub const LEVEL_WRITER: &str = "writer";
pub const LEVEL_MODERATOR: &str = "moderator";

pub fn level_to_db(level: Level) -> &'static str {
    match level {
        Level::ReadOnly => return LEVEL_READ_ONLY,
        Level::Probational => return LEVEL_PROBATIONAL,
        Level::Writer => return LEVEL_WRITER,
        Level::Moderator => return LEVEL_MODERATOR,
    }
}

pub fn level_from_db(level: &str) -> Result<Level, loga::Error> {
    match level {
        LEVEL_READ_ONLY => return Ok(Level::ReadOnly),
        LEVEL_PROBATIONAL => return Ok(Level::Probational),
        LEVEL_WRITER => return Ok(Level::Writer),
        LEVEL_MODERATOR => return Ok(Level::Moderator),
        other => return Err(loga::err(format!("Unknown stored level [{}]", other))),
    }
}

pub fn level_from_db_opt(level: Option<String>) -> Result<Option<Level>, loga::Error> {
    return level.as_deref().map(level_from_db).transpose();
}

pub const MOD_STATE_PENDING: &str = "pending";
pub const MOD_STATE_APPROVED: &str = "approved";
pub const MOD_STATE_REJECTED: &str = "rejected";
pub const REL_DELETED: &str = "deleted";

fn mod_state_to_db(state: ModerationState) -> &'static str {
    match state {
        ModerationState::Pending => return MOD_STATE_PENDING,
        ModerationState::Approved => return MOD_STATE_APPROVED,
        ModerationState::Rejected => return MOD_STATE_REJECTED,
    }
}

fn mod_state_from_db(state: &str) -> Result<ModerationState, loga::Error> {
    match state {
        MOD_STATE_PENDING => return Ok(ModerationState::Pending),
        MOD_STATE_APPROVED => return Ok(ModerationState::Approved),
        MOD_STATE_REJECTED => return Ok(ModerationState::Rejected),
        other => return Err(loga::err(format!("Unknown stored moderation state [{}]", other))),
    }
}

pub fn member_level(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    member: DbIdentity,
) -> Result<Option<Level>, loga::Error> {
    return level_from_db_opt(good_ormning::sqlite::good_query_opt!(
        db,
        r#"select level from channel_member where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and member = ${identity_id_t = &member}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?);
}

pub fn effective_level(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    local: &LocalChannel,
    who: Option<Identity>,
) -> Result<Option<Level>, loga::Error> {
    let Some(who) = who else {
        return Ok(local.default_level);
    };
    if who == owner.0 {
        return Ok(Some(Level::Moderator));
    }
    if let Some(level) = member_level(db_tx, account, owner, channel, DbIdentity(who))? {
        return Ok(Some(level));
    }
    return Ok(local.default_level);
}

pub fn require_moderator(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    local: &LocalChannel,
    actor: Identity,
) -> Result<(), S2sError> {
    if effective_level(db_tx, account, owner, channel, local, Some(actor))? != Some(Level::Moderator) {
        return Err(S2sError::Forbidden("Caller is not a moderator of this channel".to_string()));
    }
    return Ok(());
}

fn require_below_moderator(level: Level) -> Result<(), S2sError> {
    if level >= Level::Moderator {
        return Err(S2sError::Forbidden("Only the channel owner can grant moderator access".to_string()));
    }
    return Ok(());
}

pub fn moderation_entries(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    cursor: ModerationCursor,
) -> Result<Vec<ModerationEntry>, loga::Error> {
    let limit = MODERATION_PAGE_LIMIT as i64;
    let (before, after, asc) = match cursor {
        ModerationCursor::Latest => (None, None, false),
        ModerationCursor::Before(seq) => (Some(u64_to_i64(seq)), None, false),
        ModerationCursor::After(seq) => (None, Some(u64_to_i64(seq)), true),
    };
    let rows = good_ormning::sqlite::good_query_many!(
        db,
        r#"select seq, state, receive_time, sender, sender_unique, client_id, blocks, rel_type, rel_target_sender, rel_target_unique from moderation where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and (${opt i64 = before} is null or seq < ${opt i64 = before}) and (${opt i64 = after} is null or seq > ${opt i64 = after}) order by case when ${bool = asc} then seq else -seq end limit ${i64 = limit}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let rel = match r.rel_type.as_deref() {
            None => MessageRel::None,
            Some(t) => {
                let (Some(target_sender), Some(target_unique)) = (r.rel_target_sender, r.rel_target_unique) else {
                    return Err(loga::err("Stored message relation is missing its target columns"));
                };
                let target = MessageId {
                    identity: target_sender.0,
                    unique: i64_to_u64(target_unique),
                };
                match t {
                    REL_REPLY_TO => MessageRel::ReplyTo(target),
                    REL_EDIT_OF => MessageRel::EditOf(target),
                    other => return Err(loga::err(format!("Unknown stored message relation [{}]", other))),
                }
            },
        };
        out.push(ModerationEntry {
            seq: i64_to_u64(r.seq),
            state: mod_state_from_db(&r.state)?,
            received: r.receive_time.0,
            sender: r.sender.0,
            client_id: r.client_id.map(shared::interface::shared::MessageClientId),
            id: MessageId {
                identity: r.sender.0,
                unique: i64_to_u64(r.sender_unique),
            },
            rel: rel,
            blocks: parse_blocks(&r.blocks)?,
        });
    }
    if !asc {
        out.reverse();
    }
    return Ok(out);
}

pub fn reindex_root(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    sender: DbIdentity,
    sender_unique: i64,
    rel_type: Option<&str>,
    rel_target_sender: Option<DbIdentity>,
    rel_target_unique: Option<i64>,
    changed_seq: i64,
) -> Result<(), loga::Error> {
    let (root_sender, root_unique) = match (rel_type, rel_target_sender, rel_target_unique) {
        (Some(REL_EDIT_OF), Some(s), Some(u)) => (s, u),
        _ => (sender, sender_unique),
    };
    if crate::subsystems::calendar::message_event_reindex(db_tx, account, owner, channel, root_sender, root_unique)? {
        good_ormning::sqlite::good_query!(
            db,
            r#"update channel set calendar_seq = ${opt i64 = Some(changed_seq)} where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and id = ${channel_id_t = channel}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
    }
    return Ok(());
}

enum DecideOutcome {
    Approved {
        seq: u64,
        blocks: Vec<MessageBlock>,
    },
    Tombstoned {
        seq: u64,
    },
    Unchanged,
}

pub fn members_list(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
) -> Result<Vec<ChannelMemberRes>, loga::Error> {
    let mut out = vec![];
    for r in good_ormning::sqlite::good_query_many!(
        db,
        r#"select member, level from channel_member where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))? {
        out.push(ChannelMemberRes {
            identity: r.member.0,
            level: level_from_db(&r.level)?,
        });
    }
    out.sort_by(|a, b| a.identity.cmp(&b.identity));
    return Ok(out);
}

fn require_member_editable(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    actor: Identity,
    member: Identity,
) -> Result<(), S2sError> {
    let current =
        not_found(member_level(db_tx, account, owner, channel, DbIdentity(member))?, "No such channel member")?;
    if actor != owner.0 && current >= Level::Moderator {
        return Err(S2sError::Forbidden("Only the channel owner can change other moderators".to_string()));
    }
    return Ok(());
}

fn invite_res(
    rowid: i64,
    channel: QualifiedChannelId,
    token: String,
    name: String,
    description: String,
    shown_name: Option<String>,
    shown_description: Option<String>,
    single_use: bool,
    expiry: Option<DbTimeMs>,
    level: &str,
) -> Result<ChannelInviteRes, loga::Error> {
    return Ok(ChannelInviteRes {
        id: ChannelInviteId(i64_to_u64(rowid)),
        token: QualifiedChannelInviteToken {
            channel: channel,
            token: ChannelInviteToken(token),
        },
        name: name,
        description: description,
        shown_name: shown_name,
        shown_description: shown_description,
        single_use: single_use,
        expiry: expiry.map(|t| t.0),
        level: level_from_db(level)?,
    });
}

fn invite_get(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    rowid: i64,
) -> Result<Option<ChannelInviteRes>, loga::Error> {
    let Some(r) = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select rowid, identity, channel, token, name, description, shown_name, shown_description, single_use, expiry, level from channel_invite where account_id = ${account_id_t = &account} and rowid = ${i64 = rowid}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))? else {
        return Ok(None);
    };
    return Ok(Some(invite_res(r.rowid, QualifiedChannelId {
        identity: r.identity.0,
        channel: r.channel.0,
    }, r.token, r.name, r.description, r.shown_name, r.shown_description, r.single_use, r.expiry, &r.level)?));
}

pub fn invites_list(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    channel: Option<&QualifiedChannelId>,
) -> Result<Vec<ChannelInviteRes>, loga::Error> {
    let db_owner = channel.map(|c| DbIdentity(c.identity));
    let db_channel_id = channel.map(|c| DbChannelId(c.channel.clone()));
    let rows = good_ormning::sqlite::good_query_many!(
        db,
        r#"select rowid, identity, channel, token, name, description, shown_name, shown_description, single_use, expiry, level from channel_invite where account_id = ${account_id_t = &account} and (${opt identity_id_t = db_owner.as_ref()} is null or (identity = ${opt identity_id_t = db_owner.as_ref()} and channel = ${opt channel_id_t = db_channel_id.as_ref()})) order by rowid"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(invite_res(r.rowid, QualifiedChannelId {
            identity: r.identity.0,
            channel: r.channel.0,
        }, r.token, r.name, r.description, r.shown_name, r.shown_description, r.single_use, r.expiry, &r.level)?);
    }
    return Ok(out);
}

fn require_invite_in_channel(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    channel: &QualifiedChannelId,
    actor: Identity,
    rowid: i64,
) -> Result<(), S2sError> {
    let existing = not_found(invite_get(db_tx, account, rowid)?, "No such channel invite")?;
    if existing.token.channel != *channel {
        return Err(S2sError::NotFound("No such channel invite".to_string()));
    }
    if actor != channel.identity {
        require_below_moderator(existing.level)?;
    }
    return Ok(());
}

pub async fn publish_moderation_change(state: &Arc<State>, account: DbAccountId, channel: &QualifiedChannelId) {
    state.ws_bus.publish(account.0, WsEvent::Moderation(ModerationChange { channel: channel.clone() })).await;
}

pub async fn moderate(
    state: &Arc<State>,
    account: DbAccountId,
    channel: &QualifiedChannelId,
    actor: Identity,
    req: ModerateReq,
) -> Result<ModerateRes, S2sError> {
    let db_owner = DbIdentity(channel.identity);
    let db_channel_id = DbChannelId(channel.channel.clone());
    let owner_check = {
        let db_channel_id = db_channel_id.clone();
        move |db_tx: &mut db::Db<Transaction<'_>>| -> Result<LocalChannel, S2sError> {
            let local =
                not_found(
                    local_channel_get(db_tx, account, db_owner, &db_channel_id)?,
                    "Channel not found on this server",
                )?;
            if !local.is_owner(db_owner.0) {
                return Err(S2sError::NotFound("Channel is not hosted on this server".to_string()));
            }
            require_moderator(db_tx, account, db_owner, &db_channel_id, &local, actor)?;
            return Ok(local);
        }
    };
    match req {
        ModerateReq::Decide { message, approve } => {
            let (local, outcome) = s2s_tx(&state.db, "moderating", {
                let db_channel_id = db_channel_id.clone();
                move |db_tx| {
                    let local = owner_check(db_tx)?;
                    let channel = &db_channel_id;
                    let owner = db_owner;
                    let sender = DbIdentity(message.identity);
                    let sender_unique = u64_to_i64(message.unique);
                    let row = not_found(good_ormning::sqlite::good_query_opt!(
                        db,
                        r#"select state, raw from moderation where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and sender = ${identity_id_t = &sender} and sender_unique = ${i64 = sender_unique}"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?, "No such message in the moderation queue")?;
                    let state = mod_state_from_db(&row.state)?;
                    let set_state =
                        |db_tx: &mut db::Db<Transaction<'_>>, new_state: ModerationState| ->
                            Result<(), loga::Error> {
                            good_ormning::sqlite::good_query!(
                                db,
                                r#"update moderation set state = ${string = mod_state_to_db(new_state)} where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and sender = ${identity_id_t = &sender} and sender_unique = ${i64 = sender_unique}"#;
                                db_tx
                            ).map_err(|e| loga::err(e.0))?;
                            return Ok(());
                        };
                    let outcome;
                    if approve {
                        if state == ModerationState::Approved {
                            return Ok((local, DecideOutcome::Unchanged));
                        }
                        good_ormning::sqlite::good_query!(
                            db,
                            r#"delete from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and sender = ${identity_id_t = &sender} and sender_unique = ${i64 = sender_unique} and rel_type = ${string = REL_DELETED}"#;
                            db_tx
                        ).map_err(|e| loga::err(e.0))?;
                        let message =
                            serde_json::from_str::<Message>(
                                &row.raw,
                            ).context("Stored moderation message was not valid JSON")?;
                        let body =
                            serde_json::from_str::<MessageBody>(
                                &message.0.message,
                            ).context("Stored moderation message body was not valid JSON")?;
                        let seq =
                            ingest_message(
                                db_tx,
                                account,
                                owner,
                                channel,
                                &row.raw,
                                &body,
                            )?.context("Approved message could not be ingested")?;
                        set_state(db_tx, ModerationState::Approved)?;
                        outcome = DecideOutcome::Approved {
                            seq: seq,
                            blocks: body.blocks,
                        };
                    } else {
                        if state == ModerationState::Rejected {
                            return Ok((local, DecideOutcome::Unchanged));
                        }
                        let existing = good_ormning::sqlite::good_query_opt!(
                            db,
                            r#"select rel_type, rel_target_sender, rel_target_unique from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and sender = ${identity_id_t = &sender} and sender_unique = ${i64 = sender_unique}"#;
                            db_tx
                        ).map_err(|e| loga::err(e.0))?;
                        let mut tombstoned = None;
                        if let Some(existing) = existing {
                            if existing.rel_type.as_deref() != Some(REL_DELETED) {
                                let mut targets = vec![(sender, sender_unique)];
                                if existing.rel_type.as_deref() != Some(REL_EDIT_OF) {
                                    for e in good_ormning::sqlite::good_query_many!(
                                        db,
                                        r#"select sender, sender_unique from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and rel_type = ${string = REL_EDIT_OF} and rel_target_sender = ${identity_id_t = &sender} and rel_target_unique = ${i64 = sender_unique} order by seq"#;
                                        db_tx
                                    ).map_err(|e| loga::err(e.0))? {
                                        targets.push((e.sender, e.sender_unique));
                                    }
                                }
                                let mut seq = local.latest_seq.map(|s| s + 1).unwrap_or(0);
                                for (i, (t_sender, t_unique)) in targets.iter().enumerate() {
                                    good_ormning::sqlite::good_query!(
                                        db,
                                        r#"update message set seq = ${i64 = seq}, snap_offset = ${opt i64 = None}, raw = ${string = ""}, blocks = ${string = "[]"}, rel_type = ${opt string = Some(REL_DELETED)}, rel_target_sender = ${opt identity_id_t = None}, rel_target_unique = ${opt i64 = None} where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and sender = ${identity_id_t = t_sender} and sender_unique = ${i64 = *t_unique}"#;
                                        db_tx
                                    ).map_err(|e| loga::err(e.0))?;
                                    good_ormning::sqlite::good_query!(
                                        db,
                                        r#"delete from message_file where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and source = ${string = MESSAGE_FILE_SOURCE_MESSAGE} and sender = ${identity_id_t = t_sender} and sender_unique = ${i64 = *t_unique}"#;
                                        db_tx
                                    ).map_err(|e| loga::err(e.0))?;
                                    if i > 0 {
                                        good_ormning::sqlite::good_query!(
                                            db,
                                            r#"update moderation set state = ${string = MOD_STATE_REJECTED} where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and sender = ${identity_id_t = t_sender} and sender_unique = ${i64 = *t_unique}"#;
                                            db_tx
                                        ).map_err(|e| loga::err(e.0))?;
                                    }
                                    tombstoned = Some(seq);
                                    seq += 1;
                                }
                                let last_seq = tombstoned.unwrap();
                                good_ormning::sqlite::good_query!(
                                    db,
                                    r#"update channel set latest_seq = ${opt i64 = Some(last_seq)} where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and id = ${channel_id_t = channel}"#;
                                    db_tx
                                ).map_err(|e| loga::err(e.0))?;
                                reindex_root(
                                    db_tx,
                                    account,
                                    owner,
                                    channel,
                                    sender,
                                    sender_unique,
                                    existing.rel_type.as_deref(),
                                    existing.rel_target_sender,
                                    existing.rel_target_unique,
                                    last_seq,
                                )?;
                            }
                        }
                        set_state(db_tx, ModerationState::Rejected)?;
                        outcome = match tombstoned {
                            Some(seq) => DecideOutcome::Tombstoned { seq: i64_to_u64(seq) },
                            None => DecideOutcome::Unchanged,
                        };
                    }
                    return Ok((local, outcome));
                }
            }).await?;
            publish_moderation_change(state, account, channel).await;
            match outcome {
                DecideOutcome::Approved { seq, blocks } => {
                    publish_local_notification(state, account.0, channel.clone(), seq, blocks, true).await;
                },
                DecideOutcome::Tombstoned { seq } => {
                    publish_local_notification(state, account.0, channel.clone(), seq, vec![], false).await;
                },
                DecideOutcome::Unchanged => return Ok(ModerateRes::None),
            }
            let members = tx(&state.db, {
                let db_channel_id = db_channel_id.clone();
                move |db_tx| {
                    return channel_members(db_tx, account, db_owner, &db_channel_id);
                }
            }).await.context("Error listing channel members for rebroadcast")?;
            notify_remotes(state, account.0, local.local_identity, channel.clone(), members).await;
            return Ok(ModerateRes::None);
        },
        ModerateReq::MemberSet { member, level } => {
            s2s_tx(&state.db, "moderating", {
                let db_channel_id = db_channel_id.clone();
                move |db_tx| {
                    owner_check(db_tx)?;
                    require_member_editable(db_tx, account, db_owner, &db_channel_id, actor, member)?;
                    if actor != db_owner.0 {
                        require_below_moderator(level)?;
                    }
                    let db_member = DbIdentity(member);
                    good_ormning::sqlite::good_query!(
                        db,
                        r#"update channel_member set level = ${string = level_to_db(level)} where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and member = ${identity_id_t = &db_member}"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?;
                    return Ok(());
                }
            }).await?;
            let key = PullKey {
                account_id: account.0,
                owner: channel.identity,
                channel: channel.channel.clone(),
                source: member,
            };
            if level.can_post() {
                state.pull_scheduler.add_or_reset(key).await;
            } else {
                state.pull_scheduler.remove(&key).await;
            }
            return Ok(ModerateRes::None);
        },
        ModerateReq::MemberRemove { member } => {
            s2s_tx(&state.db, "moderating", {
                let db_channel_id = db_channel_id.clone();
                move |db_tx| {
                    owner_check(db_tx)?;
                    if actor != db_owner.0 {
                        require_member_editable(db_tx, account, db_owner, &db_channel_id, actor, member)?;
                    }
                    let db_member = DbIdentity(member);
                    good_ormning::sqlite::good_query!(
                        db,
                        r#"delete from channel_member where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and member = ${identity_id_t = &db_member}"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?;
                    return Ok(());
                }
            }).await?;
            state.pull_scheduler.remove(&PullKey {
                account_id: account.0,
                owner: channel.identity,
                channel: channel.channel.clone(),
                source: member,
            }).await;
            return Ok(ModerateRes::None);
        },
        ModerateReq::InviteCreate(req) => {
            if req.channel != *channel {
                return Err(S2sError::BadRequest("Invite channel doesn't match the request channel".to_string()));
            }
            let res = s2s_tx(&state.db, "moderating", move |db_tx| {
                owner_check(db_tx)?;
                if actor != db_owner.0 {
                    require_below_moderator(req.level)?;
                }
                let idem = req.idem.clone().unwrap_or_default();
                if !idem.is_empty() {
                    let existing = good_ormning::sqlite::good_query_opt!(
                        db,
                        r#"select rowid from channel_invite where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and idem = ${string = idem.as_str()}"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?;
                    if let Some(rowid) = existing {
                        return Ok(
                            invite_get(db_tx, account, rowid)?.expect("Invite row vanished within transaction"),
                        );
                    }
                }
                let token = random_token();
                good_ormning::sqlite::good_query!(
                    db,
                    r#"insert into channel_invite (account_id, identity, channel, idem, token, name, description, shown_name, shown_description, single_use, expiry, used_count, level) values (${account_id_t = &account}, ${identity_id_t = &db_owner}, ${channel_id_t = &db_channel_id}, ${string = idem.as_str()}, ${string = token.as_str()}, ${string = req.name.as_str()}, ${string = req.description.as_str()}, ${opt string = req.shown_name.as_deref()}, ${opt string = req.shown_description.as_deref()}, ${bool = req.single_use}, ${opt time_ms_t = req.expiry.map(DbTimeMs).as_ref()}, ${i64 = 0}, ${string = level_to_db(req.level)})"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
                let rowid = db_tx.0.last_insert_rowid();
                return Ok(invite_get(db_tx, account, rowid)?.expect("Invite row vanished within transaction"));
            }).await?;
            return Ok(ModerateRes::Invite(res));
        },
        ModerateReq::InviteModify(req) => {
            if req.channel != *channel {
                return Err(S2sError::BadRequest("Invite channel doesn't match the request channel".to_string()));
            }
            let res = s2s_tx(&state.db, "moderating", move |db_tx| {
                owner_check(db_tx)?;
                let rowid = u64_to_i64(req.id.0);
                require_invite_in_channel(db_tx, account, &req.channel, actor, rowid)?;
                if let Some(v) = &req.name {
                    good_ormning::sqlite::good_query!(
                        db,
                        r#"update channel_invite set name = ${string = v.as_str()} where account_id = ${account_id_t = &account} and rowid = ${i64 = rowid}"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?;
                }
                if let Some(v) = &req.description {
                    good_ormning::sqlite::good_query!(
                        db,
                        r#"update channel_invite set description = ${string = v.as_str()} where account_id = ${account_id_t = &account} and rowid = ${i64 = rowid}"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?;
                }
                if let Some(m) = &req.shown_name {
                    good_ormning::sqlite::good_query!(
                        db,
                        r#"update channel_invite set shown_name = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account} and rowid = ${i64 = rowid}"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?;
                }
                if let Some(m) = &req.shown_description {
                    good_ormning::sqlite::good_query!(
                        db,
                        r#"update channel_invite set shown_description = ${opt string = m.value.as_deref()} where account_id = ${account_id_t = &account} and rowid = ${i64 = rowid}"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?;
                }
                if let Some(v) = req.single_use {
                    good_ormning::sqlite::good_query!(
                        db,
                        r#"update channel_invite set single_use = ${bool = v} where account_id = ${account_id_t = &account} and rowid = ${i64 = rowid}"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?;
                }
                if let Some(m) = &req.expiry {
                    let v = m.value;
                    good_ormning::sqlite::good_query!(
                        db,
                        r#"update channel_invite set expiry = ${opt time_ms_t = v.map(DbTimeMs).as_ref()} where account_id = ${account_id_t = &account} and rowid = ${i64 = rowid}"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?;
                }
                if let Some(v) = req.level {
                    if actor != db_owner.0 {
                        require_below_moderator(v)?;
                    }
                    good_ormning::sqlite::good_query!(
                        db,
                        r#"update channel_invite set level = ${string = level_to_db(v)} where account_id = ${account_id_t = &account} and rowid = ${i64 = rowid}"#;
                        db_tx
                    ).map_err(|e| loga::err(e.0))?;
                }
                return Ok(invite_get(db_tx, account, rowid)?.expect("Invite row vanished within transaction"));
            }).await?;
            return Ok(ModerateRes::Invite(res));
        },
        ModerateReq::InviteDelete(req) => {
            if req.channel != *channel {
                return Err(S2sError::BadRequest("Invite channel doesn't match the request channel".to_string()));
            }
            s2s_tx(&state.db, "moderating", move |db_tx| {
                owner_check(db_tx)?;
                let rowid = u64_to_i64(req.id.0);
                require_invite_in_channel(db_tx, account, &req.channel, actor, rowid)?;
                good_ormning::sqlite::good_query!(
                    db,
                    r#"delete from channel_invite where account_id = ${account_id_t = &account} and rowid = ${i64 = rowid}"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
                return Ok(());
            }).await?;
            return Ok(ModerateRes::None);
        },
    }
}

pub async fn c2s_channel(
    state: &Arc<State>,
    account: DbAccountId,
    channel: &QualifiedChannelId,
) -> Result<LocalChannel, loga::Error> {
    let db_owner = DbIdentity(channel.identity);
    let db_channel_id = DbChannelId(channel.channel.clone());
    return Ok(tx(&state.db, move |db_tx| {
        return local_channel_get(db_tx, account, db_owner, &db_channel_id);
    }).await.context("Error looking up channel")?.context("No such channel")?);
}

pub async fn c2s_signer(
    state: &Arc<State>,
    account: DbAccountId,
    local: &LocalChannel,
) -> Result<crate::subsystems::s2s_client::Signer, loga::Error> {
    return Ok(
        crate::subsystems::messages::channel_signer(state, account.0, local)
            .await?
            .context("Channel has no authenticated identity for moderation requests")?,
    );
}

pub async fn c2s_moderate(
    state: &Arc<State>,
    account: DbAccountId,
    channel: &QualifiedChannelId,
    req: ModerateReq,
) -> Result<ModerateRes, loga::Error> {
    let local = c2s_channel(state, account, channel).await?;
    if local.is_owner(channel.identity) {
        return moderate(state, account, channel, channel.identity, req).await.map_err(|e| match e {
            S2sError::BadRequest(m) => loga::err(m),
            S2sError::Unauthorized(m) => loga::err(m),
            S2sError::Forbidden(m) => loga::err(m),
            S2sError::NotFound(m) => loga::err(m),
            S2sError::Internal(e) => e,
        });
    }
    let signer = c2s_signer(state, account, &local).await?;
    return state.s2s_client.post_authed(channel.identity, &signer, &s2sv1::Req::Moderate(Moderate {
        channel: channel.clone(),
        req: req,
    })).await;
}

pub fn own_level(local_identity: Identity, owner: Identity, stored: Option<Level>) -> Level {
    if local_identity == owner {
        return Level::Moderator;
    }
    return stored.unwrap_or(Level::ReadOnly);
}
