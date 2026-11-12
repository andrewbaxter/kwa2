use {
    crate::{
        db,
        dbutil::tx,
        interface::db::{
            DbAccountId,
            DbChannelId,
            DbIdentity,
            DbTimeMs,
        },
        subsystems::messages::{
            i64_to_u64,
            parse_blocks,
            MsgRow,
            REL_EDIT_OF,
            REL_REPLY_TO,
        },
        State,
    },
    jiff::Timestamp,
    loga::ResultContext,
    rusqlite::Transaction,
    shared::{
        calendar::{
            event_envelope,
            event_occurrences,
            month_window_utc,
            utc_midnight,
            Occurrence,
        },
        interface::{
            shared::{
                MessageBlock,
                AccountId,
                Message,
                MessageBlockEvent,
                MessageBody,
                QualifiedChannelId,
                QualifiedMessageId,
            },
            wire::c2s::{
                CalendarMessage,
                CalendarPage,
                CalendarMonth,
                CalendarToken,
            },
        },
    },
    spaghettinuum::interface::identity::Identity,
    std::sync::Arc,
};

pub type CalendarCacheKey = (AccountId, QualifiedChannelId, CalendarMonth, CalendarToken);
pub type CalendarCache = moka::sync::Cache<CalendarCacheKey, Arc<Vec<(Identity, i64)>>>;
pub type CalendarPageCache =
    moka::future::Cache<(AccountId, QualifiedChannelId, CalendarMonth, usize, CalendarToken), CalendarPage>;

pub fn message_event_reindex(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    root_sender: DbIdentity,
    root_unique: i64,
) -> Result<bool, loga::Error> {
    good_ormning::sqlite::good_query!(
        db,
        r#"delete from message_event where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and root_sender = ${identity_id_t = &root_sender} and root_unique = ${i64 = root_unique}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    let mut changed = db_tx.0.changes() > 0;
    let Some(original) = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select seq, blocks from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and sender = ${identity_id_t = &root_sender} and sender_unique = ${i64 = root_unique} and (rel_type is null or rel_type = ${string = REL_REPLY_TO})"#;
        db_tx
    ).map_err(|e| loga::err(e.0))? else {
        return Ok(changed);
    };
    let edit = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select seq, blocks from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and rel_type = ${string = REL_EDIT_OF} and rel_target_sender = ${identity_id_t = &root_sender} and rel_target_unique = ${i64 = root_unique} order by seq desc limit ${i64 = 1}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    let (version_seq, blocks) = match edit {
        Some(e) => (e.seq, e.blocks),
        None => (original.seq, original.blocks),
    };
    for (block_index, block) in parse_blocks(&blocks)?.into_iter().enumerate() {
        let MessageBlock::Event(event) = block else {
            continue;
        };
        let (envelope_start, envelope_end) = event_envelope(&event).map_err(loga::err)?;
        let event_json = serde_json::to_string(&event).unwrap();
        let block_index = block_index as i64;
        good_ormning::sqlite::good_query!(
            db,
            r#"insert into message_event (account_id, identity, channel, root_sender, root_unique, block_index, version_seq, envelope_start, envelope_end, event) values (${account_id_t = &account}, ${identity_id_t = &owner}, ${channel_id_t = channel}, ${identity_id_t = &root_sender}, ${i64 = root_unique}, ${i64 = block_index}, ${i64 = version_seq}, ${time_ms_t = &DbTimeMs(envelope_start)}, ${opt time_ms_t = envelope_end.map(DbTimeMs).as_ref()}, ${string = event_json.as_str()})"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        changed = true;
    }
    return Ok(changed);
}

pub struct EventRow {
    pub root_sender: Identity,
    pub root_unique: i64,
    pub version_seq: i64,
    pub event: MessageBlockEvent,
}

pub fn events_overlapping(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    owner: DbIdentity,
    channel: &DbChannelId,
    start: Timestamp,
    end: Timestamp,
) -> Result<Vec<EventRow>, loga::Error> {
    let rows = good_ormning::sqlite::good_query_many!(
        db,
        r#"select root_sender, root_unique, version_seq, event from message_event where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &owner} and channel = ${channel_id_t = channel} and envelope_start < ${time_ms_t = &DbTimeMs(end)} and (envelope_end is null or envelope_end > ${time_ms_t = &DbTimeMs(start)})"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push(EventRow {
            root_sender: r.root_sender.0,
            root_unique: r.root_unique,
            version_seq: r.version_seq,
            event: serde_json::from_str(&r.event).context("Stored event was not valid JSON")?,
        });
    }
    return Ok(out);
}

pub struct CalendarRoot {
    pub original: MsgRow,
    pub edit: Option<MsgRow>,
}

pub enum CalendarLocalPage {
    Stale,
    Page {
        token: CalendarToken,
        latest_seq: Option<u64>,
        roots: Vec<CalendarRoot>,
        last: bool,
    },
}

pub async fn calendar_local_page(
    state: &Arc<State>,
    account: DbAccountId,
    channel: &QualifiedChannelId,
    page_size: usize,
    month: CalendarMonth,
    page: usize,
    want_token: Option<CalendarToken>,
) -> Result<CalendarLocalPage, loga::Error> {
    let db_owner = DbIdentity(channel.identity);
    let db_channel_id = DbChannelId(channel.channel.clone());
    let cache = state.calendar_cache.clone();
    let channel = channel.clone();
    return tx(&state.db, move |db_tx| {
        let channel_row = good_ormning::sqlite::good_query_opt!(
            db,
            r#"select latest_seq, calendar_seq from channel where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and id = ${channel_id_t = &db_channel_id} and deleted is null"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?.context("Channel missing while computing calendar page")?;
        let floor = good_ormning::sqlite::good_query_opt!(
            db,
            r#"select seq from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} order by seq asc limit ${i64 = 1}"#;
            db_tx
        ).map_err(|e| loga::err(e.0))?;
        let token = CalendarToken {
            latest: channel_row.calendar_seq.map(i64_to_u64),
            floor: floor.map(i64_to_u64),
        };
        if let Some(want) = want_token {
            if want != token {
                return Ok(CalendarLocalPage::Stale);
            }
        }
        let cache_key = (account.0, channel, month, token);
        let roots = match cache.get(&cache_key) {
            Some(r) => r,
            None => {
                let window = month_window_utc(month.year, month.month).map_err(loga::err)?;
                let events = events_overlapping(db_tx, account, db_owner, &db_channel_id, window.start, window.end)?;
                let mut keyed = vec![];
                for e in events {
                    let occurrences = event_occurrences(&e.event, &window).map_err(loga::err)?;
                    let mut first = None;
                    for o in occurrences {
                        let start = match o {
                            Occurrence::AllDay { start, .. } => utc_midnight(start).map_err(loga::err)?,
                            Occurrence::Timed { start, .. } => start,
                        };
                        if first.map(|f| start < f).unwrap_or(true) {
                            first = Some(start);
                        }
                    }
                    let Some(first) = first else {
                        continue;
                    };
                    keyed.push((first, e.root_sender, e.root_unique));
                }
                keyed.sort();
                let mut seen = std::collections::HashSet::new();
                let mut roots = vec![];
                for (_, sender, unique) in keyed {
                    if seen.insert((sender, unique)) {
                        roots.push((sender, unique));
                    }
                }
                let roots = Arc::new(roots);
                cache.insert(cache_key, roots.clone());
                roots
            },
        };
        let start = page * page_size;
        let last = start + page_size >= roots.len();
        let mut out = vec![];
        for (sender, unique) in roots.iter().skip(start).take(page_size) {
            let sender = DbIdentity(*sender);
            let unique = *unique;
            let r = good_ormning::sqlite::good_query_opt!(
                db,
                r#"select sender, sender_unique, seq, snap_offset, receive_time, client_id, raw, blocks, rel_type, rel_target_sender, rel_target_unique from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and sender = ${identity_id_t = &sender} and sender_unique = ${i64 = unique}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?.context("Indexed calendar root missing from message table")?;
            let edit = good_ormning::sqlite::good_query_opt!(
                db,
                r#"select sender, sender_unique, seq, snap_offset, receive_time, client_id, raw, blocks, rel_type, rel_target_sender, rel_target_unique from message where account_id = ${account_id_t = &account} and identity = ${identity_id_t = &db_owner} and channel = ${channel_id_t = &db_channel_id} and rel_type = ${string = REL_EDIT_OF} and rel_target_sender = ${identity_id_t = &sender} and rel_target_unique = ${i64 = unique} order by seq desc limit ${i64 = 1}"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
            out.push(CalendarRoot {
                original: MsgRow {
                    sender: r.sender.0,
                    sender_unique: r.sender_unique,
                    seq: r.seq,
                    snap_offset: r.snap_offset,
                    receive_time: r.receive_time.0,
                    client_id: r.client_id,
                    raw: r.raw,
                    blocks: r.blocks,
                    rel_type: r.rel_type,
                    rel_target_sender: r.rel_target_sender.map(|s| s.0),
                    rel_target_unique: r.rel_target_unique,
                },
                edit: edit.map(|e| MsgRow {
                    sender: e.sender.0,
                    sender_unique: e.sender_unique,
                    seq: e.seq,
                    snap_offset: e.snap_offset,
                    receive_time: e.receive_time.0,
                    client_id: e.client_id,
                    raw: e.raw,
                    blocks: e.blocks,
                    rel_type: e.rel_type,
                    rel_target_sender: e.rel_target_sender.map(|s| s.0),
                    rel_target_unique: e.rel_target_unique,
                }),
            });
        }
        return Ok(CalendarLocalPage::Page {
            token: token,
            latest_seq: channel_row.latest_seq.map(i64_to_u64),
            roots: out,
            last: last,
        });
    }).await.context("Error computing calendar page");
}

pub fn calendar_message(
    original: MessageBody,
    original_receive_time: Timestamp,
    edit: Option<MessageBody>,
) -> CalendarMessage {
    let client_id = edit.as_ref().and_then(|e| e.client_id.clone()).or_else(|| original.client_id.clone());
    let original_id = QualifiedMessageId {
        channel: original.channel.clone(),
        message: original.id.clone(),
    };
    let blocks = match edit {
        Some(e) => e.blocks,
        None => original.blocks.clone(),
    };
    return CalendarMessage {
        original_id: original_id,
        original_receive_time: original_receive_time,
        client_id: client_id,
        message: MessageBody {
            blocks: blocks,
            ..original
        },
    };
}

pub fn stored_body(raw: &str) -> Result<MessageBody, loga::Error> {
    let message = serde_json::from_str::<Message>(raw).context("Stored message was not valid JSON")?;
    return Ok(
        serde_json::from_str::<MessageBody>(&message.0.message).context("Stored message body was not valid JSON")?,
    );
}
