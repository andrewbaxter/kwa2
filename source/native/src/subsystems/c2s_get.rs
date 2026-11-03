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
            s2s::SnapLookupBy,
        },
        subsystems::messages::{
            activity_rows,
            earliest_snap_offset,
            i64_to_u64,
            latest_activity_map,
            local_channel_get,
            member_fetch_activity_range,
            member_fetch_snap_lookup,
            member_fetch_snap_page,
            member_fetch_snap_page_containing_time,
            member_refresh_info,
            snap_offset_containing_time,
            snap_page_rows,
            snap_position_by_client_id,
            snap_position_by_id,
            u64_to_i64,
            LocalChannel,
            MsgRow,
            SnapPosition,
            REL_EDIT_OF,
            REL_REPLY_TO,
        },
        subsystems::s2s_error::{
            CachePolicy,
            Cached,
            SNAP_PAGE_MAX_AGE_SECS,
        },
        State,
    },
    http::{
        header::{
            CACHE_CONTROL,
            CONTENT_SECURITY_POLICY,
            CONTENT_TYPE,
            X_CONTENT_TYPE_OPTIONS,
        },
        Response,
    },
    htwrap::htserve::{
        responses::{
            body_full,
            response_400,
            response_404,
            response_503,
            Body,
        },
        viserr::VisErr,
    },
    loga::ResultContext,
    serde::Serialize,
    shared::interface::{
        shared::{
            AccountId,
            Message,
            MessageBody,
            MessageClientId,
            MessageId,
            MessageRel,
            QualifiedChannelId,
            QualifiedMessageId,
        },
        wire::c2s::{
            ActivityLatestAll,
            ActivityOffset,
            ActivityOffsetPos,
            ActivityPage,
            ActivityPageMessage,
            ActivityPageOffsetPos,
            ActivityPageRes,
            GetActivityPage,
            GetFile,
            GetJoinPreviewFile,
            GetResp,
            GetSnapPage,
            MessageIdByClientId,
            NotificationServerKey,
            OutboxEntry,
            OutboxList,
            PagePosition,
            PathReqTrait,
            SnapByClientId,
            SnapById,
            SnapByRes,
            SnapMessage,
            SnapOffset,
            SnapOffsetPos,
            SnapPage,
            SnapPageContainingTime,
            SnapPageOffsetPos,
            SnapPageRes,
        },
    },
    std::{
        collections::{
            BTreeMap,
            HashMap,
        },
        sync::Arc,
    },
};

pub enum GetErr {
    Local(loga::Error),
    Remote(loga::Error),
}

impl From<loga::Error> for GetErr {
    fn from(e: loga::Error) -> Self {
        return GetErr::Local(e);
    }
}

fn remote<T>(r: Result<T, loga::Error>) -> Result<T, GetErr> {
    return r.map_err(GetErr::Remote);
}

fn json_response<T: Serialize>(v: &T, policy: CachePolicy) -> Response<Body> {
    return Response::builder()
        .status(200)
        .header(CONTENT_TYPE, "application/json")
        .header(CACHE_CONTROL, policy.header_value())
        .body(body_full(serde_json::to_vec(v).unwrap()))
        .unwrap();
}

fn json_or_err<T: Serialize>(state: &State, what: &str, r: Result<Cached<T>, GetErr>) -> Response<Body> {
    match r {
        Ok(v) => return json_response(&GetResp::Ok(v.value), v.policy),
        Err(GetErr::Local(e)) => {
            state.log.log_err(loga::WARN, e.context(format!("Error computing {}", what)));
            return response_503();
        },
        Err(GetErr::Remote(e)) => {
            state.log.log_err(loga::DEBUG, e.context(format!("Channel owner unreachable computing {}", what)));
            return json_response(&GetResp::<T>::OwnerUnreachable, CachePolicy::NoStore);
        },
    }
}

fn no_store<T, E>(r: Result<T, E>) -> Result<Cached<T>, E> {
    return r.map(Cached::no_store);
}

pub async fn handle(
    state: &Arc<State>,
    account: AccountId,
    remainder: &str,
) -> Result<Response<Body>, VisErr<loga::Error>> {
    let first = remainder.trim_start_matches('/').split('/').next().unwrap().to_string();
    match first.as_str() {
        "notification_server_key" => {
            if NotificationServerKey::deserialize_path(remainder).is_err() {
                return Ok(response_400("Bad path".to_string()));
            }
            return Ok(json_response(&GetResp::Ok(&state.vapid_keys.public_b64url), CachePolicy::NoStore));
        },
        "file" => {
            let req =
                match GetFile::deserialize_path(remainder) {
                    Ok(r) => r,
                    Err(e) => return Ok(response_400(e)),
                };
            let result =
                match crate::subsystems::files::read_file(state, DbAccountId(account), &req.hash).await {
                    Ok(v) => v,
                    Err(e) => {
                        state.log.log_err(loga::WARN, e.context("Error fetching file"));
                        return Ok(response_503());
                    },
                };
            let Some((mime, bytes)) = result else {
                return Ok(response_404());
            };
            return Ok(
                Response::builder()
                    .status(200)
                    .header(CONTENT_TYPE, mime)
                    .header(X_CONTENT_TYPE_OPTIONS, "nosniff")
                    .header(CONTENT_SECURITY_POLICY, "sandbox; default-src 'none'")
                    .header(CACHE_CONTROL, CachePolicy::Immutable.header_value())
                    .body(body_full(bytes))
                    .unwrap(),
            );
        },
        "join_preview_file" => {
            let req =
                match GetJoinPreviewFile::deserialize_path(remainder) {
                    Ok(r) => r,
                    Err(e) => return Ok(response_400(e)),
                };
            let (mime, bytes) =
                match crate::subsystems::c2s::fetch_join_portrait(state, &req.url, req.hash).await {
                    Ok(v) => v,
                    Err(e) => {
                        state.log.log_err(loga::DEBUG, e.context("Error fetching join preview file"));
                        return Ok(response_404());
                    },
                };
            return Ok(
                Response::builder()
                    .status(200)
                    .header(CONTENT_TYPE, mime)
                    .header(X_CONTENT_TYPE_OPTIONS, "nosniff")
                    .header(CONTENT_SECURITY_POLICY, "sandbox; default-src 'none'")
                    .header(CACHE_CONTROL, CachePolicy::NoStore.header_value())
                    .body(body_full(bytes))
                    .unwrap(),
            );
        },
        "activity_latest_all" => {
            if ActivityLatestAll::deserialize_path(remainder).is_err() {
                return Ok(response_400("Bad path".to_string()));
            }
            return Ok(
                json_or_err(
                    state,
                    "activity_latest_all",
                    no_store(activity_latest_all(state, account).await.map_err(GetErr::Local)),
                ),
            );
        },
        "activity_page" => {
            let req = match GetActivityPage::deserialize_path(remainder) {
                Ok(r) => r,
                Err(e) => return Ok(response_400(e)),
            };
            return Ok(
                json_or_err(state, "activity_page", get_activity_page(state, account, req.channel, req.page).await),
            );
        },
        "snap_page" => {
            let req = match GetSnapPage::deserialize_path(remainder) {
                Ok(r) => r,
                Err(e) => return Ok(response_400(e)),
            };
            return Ok(json_or_err(state, "snap_page", get_snap_page(state, account, req.channel, req.page).await));
        },
        "snap_by_id" => {
            let req = match SnapById::deserialize_path(remainder) {
                Ok(r) => r,
                Err(e) => return Ok(response_400(e)),
            };
            return Ok(json_or_err(state, "snap_by_id", snap_by_qualified_id(state, account, req.id).await));
        },
        "snap_by_client_id" => {
            let req = match SnapByClientId::deserialize_path(remainder) {
                Ok(r) => r,
                Err(e) => return Ok(response_400(e)),
            };
            return Ok(
                json_or_err(
                    state,
                    "snap_by_client_id",
                    snap_by_client_id(state, account, req.channel, req.client_id).await,
                ),
            );
        },
        "snap_page_containing_time" => {
            let req = match SnapPageContainingTime::deserialize_path(remainder) {
                Ok(r) => r,
                Err(e) => return Ok(response_400(e)),
            };
            return Ok(
                json_or_err(
                    state,
                    "snap_page_containing_time",
                    no_store(snap_page_containing_time(state, account, req.channel, req.time).await),
                ),
            );
        },
        "outbox" => {
            let req = match OutboxList::deserialize_path(remainder) {
                Ok(r) => r,
                Err(e) => return Ok(response_400(e)),
            };
            return Ok(
                json_or_err(
                    state,
                    "outbox",
                    no_store(outbox_list(state, account, req.channel).await.map_err(GetErr::Local)),
                ),
            );
        },
        "message_id_by_client_id" => {
            let req = match MessageIdByClientId::deserialize_path(remainder) {
                Ok(r) => r,
                Err(e) => return Ok(response_400(e)),
            };
            return Ok(
                json_or_err(
                    state,
                    "message_id_by_client_id",
                    no_store(message_id_by_client_id(state, account, req.channel, req.client_id).await),
                ),
            );
        },
        _ => return Ok(response_404()),
    }
}

pub const GET_PATH_PREFIXES: &[&str] =
    &[
        "notification_server_key",
        "portrait",
        "file",
        "join_preview_file",
        "activity_latest_all",
        "activity_page",
        "snap_page",
        "snap_by_id",
        "snap_by_client_id",
        "snap_page_containing_time",
        "outbox",
        "message_id_by_client_id",
    ];

pub fn page_position(offset: usize, page_size: usize, floor: Option<usize>) -> PagePosition {
    let in_page = offset % page_size;
    if in_page == 0 || floor == Some(offset) {
        return PagePosition::First;
    }
    if in_page + 1 == page_size {
        return PagePosition::Last;
    }
    return PagePosition::Middle;
}

fn range_complete(count: usize, start: i64, page_size: usize, latest: Option<i64>, floor: Option<i64>) -> bool {
    let end = start + page_size as i64;
    let effective_start = match floor {
        Some(f) if f > start => f,
        _ => start,
    };
    if effective_start >= end {
        return false;
    }
    if count as i64 == end - effective_start {
        return true;
    }
    let Some(latest) = latest else {
        return false;
    };
    if latest < effective_start || latest >= end {
        return false;
    }
    return count as i64 == latest - effective_start + 1;
}

fn activity_pos(offset: usize, page_size: usize) -> ActivityPageOffsetPos {
    return ActivityPageOffsetPos {
        page: ActivityPage(offset / page_size),
        offset_pos: ActivityOffsetPos {
            offset: ActivityOffset(offset),
            pos: page_position(offset, page_size, None),
        },
    };
}

fn snap_pos(offset: usize, page_size: usize, floor: Option<usize>) -> SnapPageOffsetPos {
    return SnapPageOffsetPos {
        page: SnapPage(offset / page_size),
        offset_pos: SnapOffsetPos {
            offset: SnapOffset(offset),
            pos: page_position(offset, page_size, floor),
        },
    };
}

fn floor_usize(floor: Option<i64>) -> Option<usize> {
    return floor.map(|f| i64_to_u64(f) as usize);
}

struct ChannelCtx {
    account: DbAccountId,
    db_owner: DbIdentity,
    db_channel_id: DbChannelId,
    channel: QualifiedChannelId,
}

impl ChannelCtx {
    fn new(account: AccountId, channel: QualifiedChannelId) -> Self {
        return Self {
            account: DbAccountId(account),
            db_owner: DbIdentity(channel.identity),
            db_channel_id: DbChannelId(channel.channel.clone()),
            channel: channel,
        };
    }
}

pub async fn activity_latest_all(
    state: &Arc<State>,
    account: AccountId,
) -> Result<Vec<(QualifiedChannelId, ActivityOffset)>, loga::Error> {
    let account_id = DbAccountId(account);
    let rows = tx(&state.db, move |db_tx| {
        return latest_activity_map(db_tx, account_id);
    }).await.context("Error querying channel activity")?;
    return Ok(rows.into_iter().map(|(k, v)| (k, ActivityOffset(v as usize))).collect());
}

fn build_activity_page(
    rows: BTreeMap<i64, String>,
    page_size: usize,
) -> Result<Option<ActivityPageRes>, loga::Error> {
    if rows.is_empty() {
        return Ok(None);
    }
    let mut messages = Vec::with_capacity(rows.len());
    for (seq, raw) in rows {
        let offset = i64_to_u64(seq) as usize;
        messages.push(ActivityPageMessage {
            message: serde_json::from_str::<Message>(&raw).context("Stored message row has invalid raw JSON")?,
            offset_pos: ActivityOffsetPos {
                offset: ActivityOffset(offset),
                pos: page_position(offset, page_size, None),
            },
        });
    }
    return Ok(Some(ActivityPageRes { messages: messages }));
}

pub async fn get_activity_page(
    state: &Arc<State>,
    account: AccountId,
    channel: QualifiedChannelId,
    page: ActivityPage,
) -> Result<Cached<Option<ActivityPageRes>>, GetErr> {
    let ctx = ChannelCtx::new(account, channel);
    let Some((local, rows)) = tx(&state.db, {
        let account = ctx.account;
        let db_owner = ctx.db_owner;
        let db_channel_id = ctx.db_channel_id.clone();
        move |db_tx| {
            let Some(local) = local_channel_get(db_tx, account, db_owner, &db_channel_id)? else {
                return Ok(None);
            };
            let start = u64_to_i64(page.0 as u64 * local.page_size as u64);
            let end = start + local.page_size as i64;
            let rows = activity_rows(db_tx, account, db_owner, &db_channel_id, start, end)?;
            return Ok(Some((local, rows)));
        }
    }).await.context("Error querying activity page")? else {
        return Ok(Cached::no_store(None));
    };
    let page_size = local.page_size;
    let start = u64_to_i64(page.0 as u64 * page_size as u64);
    let mut merged: BTreeMap<i64, String> = rows.into_iter().collect();
    if !local.is_owner(ctx.channel.identity) && !range_complete(merged.len(), start, page_size, local.latest_seq, None) {
        let fetched =
            remote(
                member_fetch_activity_range(
                    state,
                    account,
                    &ctx.channel,
                    &local,
                    i64_to_u64(start),
                    i64_to_u64(start) + page_size as u64,
                ).await,
            )?;
        for v in fetched {
            merged.insert(u64_to_i64(v.seq), v.raw);
        }
    }
    let full = merged.len() == page_size;
    return Ok(Cached {
        value: build_activity_page(merged, page_size)?,
        policy: CachePolicy::Immutable.only_if(local.public_read && full),
    });
}

fn build_snap_page(
    channel: &QualifiedChannelId,
    page_size: usize,
    page: u64,
    latest_seq: Option<i64>,
    floor: Option<i64>,
    originals: BTreeMap<i64, MsgRow>,
    edits: Vec<MsgRow>,
) -> Result<Option<SnapPageRes>, loga::Error> {
    if originals.is_empty() && page > 0 {
        return Ok(None);
    }
    let mut latest_edits: HashMap<(spaghettinuum::interface::identity::Identity, i64), MsgRow> = HashMap::new();
    for e in edits {
        let (Some(target_sender), Some(target_unique)) = (e.rel_target_sender, e.rel_target_unique) else {
            return Err(loga::err("Edit row missing target columns"));
        };
        let key = (target_sender, target_unique);
        let replace = latest_edits.get(&key).map(|existing| e.seq > existing.seq).unwrap_or(true);
        if replace {
            latest_edits.insert(key, e);
        }
    }
    let mut messages = Vec::with_capacity(originals.len());
    for (_, r) in originals {
        let offset = i64_to_u64(r.snap_offset.context("Snap original without snap offset")?) as usize;
        let origin_client_id = r.client_id.clone().map(MessageClientId);
        let (final_body, final_client_id) = match latest_edits.get(&(r.sender, r.sender_unique)) {
            Some(e) => (
                e.body_text.clone(),
                e.client_id.clone().map(MessageClientId).or_else(|| origin_client_id.clone()),
            ),
            None => (r.body_text.clone(), origin_client_id.clone()),
        };
        let rel = match (r.rel_type.as_deref(), r.rel_target_sender, r.rel_target_unique) {
            (Some(REL_REPLY_TO), Some(target_sender), Some(target_unique)) => MessageRel::ReplyTo(MessageId {
                identity: target_sender,
                unique: i64_to_u64(target_unique),
            }),
            (None, _, _) => MessageRel::None,
            (Some(REL_EDIT_OF), _, _) => return Err(loga::err("Edit row stored with a snap offset")),
            _ => return Err(loga::err("Snap row has inconsistent relation columns")),
        };
        let id = MessageId {
            identity: r.sender,
            unique: i64_to_u64(r.sender_unique),
        };
        messages.push(SnapMessage {
            offset_pos: SnapOffsetPos {
                offset: SnapOffset(offset),
                pos: page_position(offset, page_size, floor_usize(floor)),
            },
            original_id: QualifiedMessageId {
                channel: channel.clone(),
                message: id.clone(),
            },
            original_receive_time: r.receive_time,
            client_id: final_client_id,
            message: MessageBody {
                channel: channel.clone(),
                client_id: origin_client_id,
                id: id,
                rel: rel,
                body: final_body,
            },
        });
    }
    return Ok(Some(SnapPageRes {
        latest_activity: latest_seq.map(|s| activity_pos(i64_to_u64(s) as usize, page_size)),
        messages: messages,
    }));
}

pub async fn get_snap_page(
    state: &Arc<State>,
    account: AccountId,
    channel: QualifiedChannelId,
    page: SnapPage,
) -> Result<Cached<Option<SnapPageRes>>, GetErr> {
    let ctx = ChannelCtx::new(account, channel);
    let page = page.0 as u64;
    let Some((local, floor, originals, edits)) = tx(&state.db, {
        let account = ctx.account;
        let db_owner = ctx.db_owner;
        let db_channel_id = ctx.db_channel_id.clone();
        move |db_tx| {
            let Some(local) = local_channel_get(db_tx, account, db_owner, &db_channel_id)? else {
                return Ok(None);
            };
            let floor = if local.is_owner(db_owner.0) {
                earliest_snap_offset(db_tx, account, db_owner, &db_channel_id)?
            } else {
                local.floor_snap_offset
            };
            let (originals, edits) = snap_page_rows(db_tx, account, db_owner, &db_channel_id, local.page_size, page)?;
            return Ok(Some((local, floor, originals, edits)));
        }
    }).await.context("Error querying snap page")? else {
        return Ok(Cached::no_store(None));
    };
    let page_size = local.page_size;
    let start = u64_to_i64(page * page_size as u64);
    let mut originals: BTreeMap<i64, MsgRow> =
        originals.into_iter().map(|r| (r.snap_offset.unwrap(), r)).collect();
    let mut edits = edits;
    let latest_seq = local.latest_seq;
    let mut floor = floor;
    if !local.is_owner(ctx.channel.identity) &&
        !range_complete(originals.len(), start, page_size, local.latest_snap_offset, floor) {
        let Some(fetched) = remote(member_fetch_snap_page(state, account, &ctx.channel, &local, page).await)? else {
            return Ok(Cached::no_store(None));
        };
        for v in &fetched {
            let row = MsgRow::from_verified(v);
            match row.snap_offset {
                Some(offset) => {
                    originals.insert(offset, row);
                },
                None => {
                    edits.push(row);
                },
            }
        }
        if originals.len() < page_size {
            let info = remote(member_refresh_info(state, account, &ctx.channel, &local).await)?;
            floor = info.earliest_snap_offset.map(u64_to_i64);
        }
    }
    return Ok(Cached {
        value: build_snap_page(&ctx.channel, page_size, page, latest_seq, floor, originals, edits)?,
        policy: CachePolicy::MaxAge(SNAP_PAGE_MAX_AGE_SECS).only_if(local.public_read),
    });
}

fn snap_by_res(pos: SnapPosition, page_size: usize, floor: Option<i64>) -> SnapByRes {
    return SnapByRes {
        original_receive_time: pos.receive_time,
        offset: snap_pos(i64_to_u64(pos.snap_offset) as usize, page_size, floor_usize(floor)),
    };
}

async fn local_channel(state: &Arc<State>, ctx: &ChannelCtx) -> Result<Option<LocalChannel>, loga::Error> {
    let account = ctx.account;
    let db_owner = ctx.db_owner;
    let db_channel_id = ctx.db_channel_id.clone();
    return tx(&state.db, move |db_tx| {
        return local_channel_get(db_tx, account, db_owner, &db_channel_id);
    }).await.context("Error loading channel");
}

fn channel_floor(
    db_tx: &mut crate::db::Db<rusqlite::Transaction<'_>>,
    account: DbAccountId,
    db_owner: DbIdentity,
    db_channel_id: &DbChannelId,
    local: &LocalChannel,
) -> Result<Option<i64>, loga::Error> {
    if local.is_owner(db_owner.0) {
        return earliest_snap_offset(db_tx, account, db_owner, db_channel_id);
    } else {
        return Ok(local.floor_snap_offset);
    }
}

pub async fn snap_by_qualified_id(
    state: &Arc<State>,
    account: AccountId,
    id: QualifiedMessageId,
) -> Result<Cached<Option<SnapByRes>>, GetErr> {
    let ctx = ChannelCtx::new(account, id.channel.clone());
    let message = id.message;
    let Some((local, floor, pos)) = tx(&state.db, {
        let account = ctx.account;
        let db_owner = ctx.db_owner;
        let db_channel_id = ctx.db_channel_id.clone();
        let message = message.clone();
        move |db_tx| {
            let Some(local) = local_channel_get(db_tx, account, db_owner, &db_channel_id)? else {
                return Ok(None);
            };
            let floor = channel_floor(db_tx, account, db_owner, &db_channel_id, &local)?;
            let pos = snap_position_by_id(db_tx, account, db_owner, &db_channel_id, &message)?;
            return Ok(Some((local, floor, pos)));
        }
    }).await.context("Error looking up snap position")? else {
        return Ok(Cached::no_store(None));
    };
    if let Some(pos) = pos {
        return Ok(Cached {
            value: Some(snap_by_res(pos, local.page_size, floor)),
            policy: CachePolicy::Immutable.only_if(local.public_read),
        });
    }
    if local.is_owner(ctx.channel.identity) {
        return Ok(Cached::no_store(None));
    }
    let Some(found) =
        remote(member_fetch_snap_lookup(state, account, &ctx.channel, &local, SnapLookupBy::Id(message)).await)? else {
            return Ok(Cached::no_store(None));
        };
    return Ok(Cached {
        value: Some(snap_by_res(SnapPosition {
            original_id: found.original_id,
            snap_offset: u64_to_i64(found.snap_offset),
            receive_time: found.receive_time,
        }, local.page_size, floor)),
        policy: CachePolicy::Immutable.only_if(local.public_read),
    });
}

pub async fn snap_by_client_id(
    state: &Arc<State>,
    account: AccountId,
    channel: QualifiedChannelId,
    client_id: MessageClientId,
) -> Result<Cached<Option<SnapByRes>>, GetErr> {
    let ctx = ChannelCtx::new(account, channel);
    let Some((local, floor, pos)) = tx(&state.db, {
        let account = ctx.account;
        let db_owner = ctx.db_owner;
        let db_channel_id = ctx.db_channel_id.clone();
        let client_id = client_id.0.clone();
        move |db_tx| {
            let Some(local) = local_channel_get(db_tx, account, db_owner, &db_channel_id)? else {
                return Ok(None);
            };
            let floor = channel_floor(db_tx, account, db_owner, &db_channel_id, &local)?;
            let pos =
                snap_position_by_client_id(
                    db_tx,
                    account,
                    db_owner,
                    &db_channel_id,
                    local.local_identity,
                    &client_id,
                )?;
            return Ok(Some((local, floor, pos)));
        }
    }).await.context("Error looking up message by client_id")? else {
        return Ok(Cached::no_store(None));
    };
    if let Some(pos) = pos {
        return Ok(Cached {
            value: Some(snap_by_res(pos, local.page_size, floor)),
            policy: CachePolicy::Immutable.only_if(local.public_read),
        });
    }
    if local.is_owner(ctx.channel.identity) {
        return Ok(Cached::no_store(None));
    }
    let Some(found) =
        remote(
            member_fetch_snap_lookup(
                state,
                account,
                &ctx.channel,
                &local,
                SnapLookupBy::ClientId(local.local_identity, client_id),
            ).await,
        )? else {
            return Ok(Cached::no_store(None));
        };
    return Ok(Cached {
        value: Some(snap_by_res(SnapPosition {
            original_id: found.original_id,
            snap_offset: u64_to_i64(found.snap_offset),
            receive_time: found.receive_time,
        }, local.page_size, floor)),
        policy: CachePolicy::Immutable.only_if(local.public_read),
    });
}

pub async fn message_id_by_client_id(
    state: &Arc<State>,
    account: AccountId,
    channel: QualifiedChannelId,
    client_id: MessageClientId,
) -> Result<Option<MessageId>, GetErr> {
    let ctx = ChannelCtx::new(account, channel);
    let Some((local, found)) = tx(&state.db, {
        let account = ctx.account;
        let db_owner = ctx.db_owner;
        let db_channel_id = ctx.db_channel_id.clone();
        let client_id = client_id.0.clone();
        move |db_tx| {
            let Some(local) = local_channel_get(db_tx, account, db_owner, &db_channel_id)? else {
                return Ok(None);
            };
            let db_sender = DbIdentity(local.local_identity);
            let outbox =
                good_ormning::sqlite::good_query_opt!(
                    db,
                    r#"select sender_unique from outbox where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and sender = ${identity_id_t = &db_sender} and client_id = ${string = client_id.as_str()}"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
            if let Some(unique) = outbox {
                return Ok(Some((local.clone(), Some(MessageId {
                    identity: local.local_identity,
                    unique: i64_to_u64(unique),
                }))));
            }
            let stored =
                good_ormning::sqlite::good_query_opt!(
                    db,
                    r#"select sender_unique from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and sender = ${identity_id_t = &db_sender} and client_id = ${string = client_id.as_str()}"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?;
            return Ok(Some((local.clone(), stored.map(|unique| MessageId {
                identity: local.local_identity,
                unique: i64_to_u64(unique),
            }))));
        }
    }).await.context("Error looking up message id by client_id")? else {
        return Ok(None);
    };
    if let Some(found) = found {
        return Ok(Some(found));
    }
    if local.is_owner(ctx.channel.identity) {
        return Ok(None);
    }
    let found =
        remote(
            member_fetch_snap_lookup(
                state,
                account,
                &ctx.channel,
                &local,
                SnapLookupBy::ClientId(local.local_identity, client_id),
            ).await,
        )?;
    return Ok(found.map(|f| f.original_id));
}

pub async fn outbox_list(
    state: &Arc<State>,
    account: AccountId,
    channel: QualifiedChannelId,
) -> Result<Vec<OutboxEntry>, loga::Error> {
    let ctx = ChannelCtx::new(account, channel);
    let rows = tx(&state.db, {
        let account = ctx.account;
        let db_owner = ctx.db_owner;
        let db_channel_id = ctx.db_channel_id.clone();
        move |db_tx| {
            return Ok(
                good_ormning::sqlite::good_query_many!(
                    db,
                    r#"select sender, client_id, created, raw from outbox where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} order by sender_unique"#;
                    db_tx
                ).map_err(|e| loga::err(e.0))?,
            );
        }
    }).await.context("Error listing outbox")?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let message = serde_json::from_str::<Message>(&r.raw).context("Stored outbox message was not valid JSON")?;
        let body =
            serde_json::from_str::<MessageBody>(&message.0.message).context("Stored outbox body was not valid JSON")?;
        out.push(OutboxEntry {
            sender: r.sender.0,
            client_id: MessageClientId(r.client_id),
            created: r.created.0,
            id: body.id,
            rel: body.rel,
            body: body.body,
        });
    }
    return Ok(out);
}

pub async fn snap_page_containing_time(
    state: &Arc<State>,
    account: AccountId,
    channel: QualifiedChannelId,
    time: jiff::Timestamp,
) -> Result<Option<SnapPage>, GetErr> {
    let ctx = ChannelCtx::new(account, channel);
    let Some(local) = local_channel(state, &ctx).await? else {
        return Ok(None);
    };
    let page_size = local.page_size;
    if local.is_owner(ctx.channel.identity) || local.backfill_done {
        let offset = tx(&state.db, {
            let account = ctx.account;
            let db_owner = ctx.db_owner;
            let db_channel_id = ctx.db_channel_id.clone();
            move |db_tx| {
                return snap_offset_containing_time(db_tx, account, db_owner, &db_channel_id, time);
            }
        }).await.context("Error finding snap page for time")?;
        if local.is_owner(ctx.channel.identity) {
            return Ok(offset.map(|o| SnapPage(i64_to_u64(o) as usize / page_size)));
        }
        if let Some(o) = offset {
            return Ok(Some(SnapPage(i64_to_u64(o) as usize / page_size)));
        }
    }
    return Ok(
        remote(member_fetch_snap_page_containing_time(state, account, &ctx.channel, &local, time).await)?
            .map(|p| SnapPage(p as usize)),
    );
}
