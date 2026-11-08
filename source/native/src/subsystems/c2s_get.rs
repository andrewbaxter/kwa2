use {
    crate::{
        interface::db::{
            DbAccountId,
            DbChannelId,
            DbIdentity,
        },
        subsystems::messages::{
            earliest_snap_offset,
            i64_to_u64,
            u64_to_i64,
            LocalChannel,
            MsgRow,
            SnapPosition,
            REL_EDIT_OF,
        },
        subsystems::s2s_error::{
            CachePolicy,
            Cached,
        },
        State,
    },
    http::{
        header::{
            CACHE_CONTROL,
            CONTENT_TYPE,
        },
        Response,
    },
    htwrap::htserve::responses::{
        body_full,
        response_503,
        Body,
    },
    serde::Serialize,
    shared::interface::{
        shared::{
            AccountId,
            MessageId,
            QualifiedChannelId,
        },
        wire::c2s::{
            GetResp,
            MessageRevision,
            PagePosition,
            SnapByRes,
            SnapOffset,
            SnapOffsetPos,
            SnapPage,
            SnapPageOffsetPos,
        },
    },
    std::collections::HashSet,
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

pub fn remote<T>(r: Result<T, loga::Error>) -> Result<T, GetErr> {
    return r.map_err(GetErr::Remote);
}

pub fn json_response<T: Serialize>(v: &T, policy: CachePolicy) -> Response<Body> {
    return Response::builder()
        .status(200)
        .header(CONTENT_TYPE, "application/json")
        .header(CACHE_CONTROL, policy.header_value())
        .body(body_full(serde_json::to_vec(v).unwrap()))
        .unwrap();
}

pub fn json_or_err<T: Serialize>(state: &State, what: &str, r: Result<Cached<T>, GetErr>) -> Response<Body> {
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

pub fn no_store<T, E>(r: Result<T, E>) -> Result<Cached<T>, E> {
    return r.map(Cached::no_store);
}

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

pub fn range_complete(count: usize, start: i64, page_size: usize, latest: Option<i64>, floor: Option<i64>) -> bool {
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

pub fn floor_usize(floor: Option<i64>) -> Option<usize> {
    return floor.map(|f| i64_to_u64(f) as usize);
}

pub struct ChannelCtx {
    pub account: DbAccountId,
    pub db_owner: DbIdentity,
    pub db_channel_id: DbChannelId,
    pub channel: QualifiedChannelId,
}

impl ChannelCtx {
    pub fn new(account: AccountId, channel: QualifiedChannelId) -> Self {
        return Self {
            account: DbAccountId(account),
            db_owner: DbIdentity(channel.identity),
            db_channel_id: DbChannelId(channel.channel.clone()),
            channel: channel,
        };
    }
}

pub fn snap_by_res(pos: SnapPosition, page_size: usize, floor: Option<i64>) -> SnapByRes {
    return SnapByRes {
        original_receive_time: pos.receive_time,
        offset: SnapPageOffsetPos {
            page: SnapPage(i64_to_u64(pos.snap_offset) as usize / page_size),
            offset_pos: SnapOffsetPos {
                offset: SnapOffset(i64_to_u64(pos.snap_offset) as usize),
                pos: page_position(i64_to_u64(pos.snap_offset) as usize, page_size, floor_usize(floor)),
            },
        },
    };
}

pub fn channel_floor(
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

pub struct OutboxEdit {
    pub created: jiff::Timestamp,
    pub client_id: String,
    pub body: String,
}

pub fn build_message_history(
    id: &MessageId,
    rows: Vec<MsgRow>,
    outbox_edits: Vec<OutboxEdit>,
) -> Option<Vec<MessageRevision>> {
    let sender_unique = u64_to_i64(id.unique);
    let mut original = None;
    let mut edits = vec![];
    for r in rows {
        match r.rel_type.as_deref() {
            Some(REL_EDIT_OF) => {
                if r.rel_target_sender == Some(id.identity) && r.rel_target_unique == Some(sender_unique) {
                    edits.push(r);
                }
            },
            _ => {
                if r.sender == id.identity && r.sender_unique == sender_unique && r.snap_offset.is_some() {
                    original = Some(r);
                }
            },
        }
    }
    let original = original?;
    edits.sort_by_key(|r| r.seq);
    let mut out = Vec::with_capacity(edits.len() + outbox_edits.len() + 1);
    out.push(MessageRevision {
        receive_time: original.receive_time,
        body: original.body_text,
    });
    let mut owner_client_ids = HashSet::new();
    for r in edits {
        if let Some(c) = &r.client_id {
            owner_client_ids.insert(c.clone());
        }
        out.push(MessageRevision {
            receive_time: r.receive_time,
            body: r.body_text,
        });
    }
    let mut extra = vec![];
    for e in outbox_edits {
        if owner_client_ids.contains(&e.client_id) {
            continue;
        }
        extra.push(MessageRevision {
            receive_time: e.created,
            body: e.body,
        });
    }
    if !extra.is_empty() {
        let first = out.remove(0);
        out.extend(extra);
        out.sort_by_key(|r| r.receive_time);
        out.insert(0, first);
    }
    return Some(out);
}
