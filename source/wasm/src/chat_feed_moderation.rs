use {
    crate::{
        api::{
            req_get_retry,
            req_post_json,
        },
        chat::ChatState2,
        blocks::view_blocks,
        chat::{
            ChatState,
            CopyStage,
            new_message_copy,
        },
        chat_entry::{
            ChatEntry,
            ChatFeedId,
            ChatTime,
            ChatTimeId,
        },
        infinite::{
            Feed,
            WeakInfinite,
        },
        state::{
            spawn_log,
            spawn_rooted_log,
            state,
        },
    },
    futures::channel::oneshot,
    gloo::{
        events::EventListener,
        timers::callback::Interval,
        utils::document,
    },
    jiff::Timestamp,
    lunk::{
        EventGraph,
        Prim,
    },
    rooting::{
        scope_any,
        ScopeValue,
    },
    shared::interface::{
        shared::{
            Level,
            QualifiedChannelId,
            QualifiedMessageId,
        },
        wire::c2s::{
            self,
            ModerationCursor,
            ModerationEntry,
            MODERATION_PAGE_LIMIT,
        },
    },
    std::{
        cell::RefCell,
        rc::Rc,
    },
    web_sys::VisibilityState,
};

struct ModerationFeed_ {
    id: QualifiedChannelId,
    chat_state: Rc<ChatState>,
    parent: RefCell<Option<WeakInfinite<ChatEntry>>>,
    pulling_around: RefCell<Option<oneshot::Receiver<()>>>,
    pulling_before: RefCell<Option<oneshot::Receiver<()>>>,
    pulling_after: RefCell<Option<oneshot::Receiver<()>>>,
}

#[derive(Clone)]
pub struct ModerationFeed(Rc<ModerationFeed_>);

const WANT_COUNT: usize = 10;

fn entry_time(e: &ModerationEntry) -> ChatTime {
    return ChatTime {
        stamp: e.received,
        id: ChatTimeId::Moderation(e.seq),
    };
}

impl ModerationFeed {
    pub fn new(chat_state: Rc<ChatState>, id: QualifiedChannelId) -> ModerationFeed {
        return ModerationFeed(Rc::new(ModerationFeed_ {
            id: id,
            chat_state: chat_state,
            parent: RefCell::new(None),
            pulling_around: RefCell::new(None),
            pulling_before: RefCell::new(None),
            pulling_after: RefCell::new(None),
        }));
    }

    fn feed_id(&self) -> ChatFeedId {
        return ChatFeedId::Moderation(self.0.id.clone());
    }

    pub fn active(&self) -> bool {
        return state()
            .lookup_channel
            .borrow()
            .get(&self.0.id)
            .map(|c| c.level.get() == Level::Moderator)
            .unwrap_or(false);
    }

    pub fn notify(&self, eg: EventGraph) {
        spawn_log("Moderation feed, syncing states", {
            let self1 = self.clone();
            async move {
                let page = self1.fetch(ModerationCursor::Latest).await?;
                self1.sync_states(&page);
                return Ok(());
            }
        });
        let Some(parent) = self.0.parent.borrow().as_ref().unwrap().upgrade() else {
            return;
        };
        let Some(pivot) = parent.want_after(self.feed_id()) else {
            return;
        };
        self.request_after(eg, pivot);
    }

    async fn fetch(&self, cursor: ModerationCursor) -> Result<Vec<ModerationEntry>, String> {
        if !self.active() {
            return Ok(vec![]);
        }
        match req_get_retry(c2s::GetModeration {
            channel: self.0.id.clone(),
            cursor: cursor,
        }).await {
            Ok(entries) => return Ok(entries),
            Err(e) => {
                state().log.log(&format!("Moderation queue request failed, re-checking level: {}", e));
                refresh_level(self.0.chat_state.eg.clone(), self.0.id.clone());
                return Ok(vec![]);
            },
        }
    }

    fn sync_states(&self, entries: &[ModerationEntry]) {
        let mut state_updates = vec![];
        {
            let mut states = self.0.chat_state.moderation_states.borrow_mut();
            for e in entries {
                let message_id = QualifiedMessageId {
                    channel: self.0.id.clone(),
                    message: e.id.clone(),
                };
                match states.get(&message_id) {
                    Some(existing) => {
                        if *existing.borrow() != e.state {
                            state_updates.push((existing.clone(), e.state));
                        }
                    },
                    None => {
                        states.insert(message_id, Prim::new(e.state));
                    },
                }
            }
        }
        if !state_updates.is_empty() {
            self.0.chat_state.eg.event(|pc| {
                for (prim, new_state) in state_updates {
                    prim.set(pc, new_state);
                }
            }).unwrap();
        }
    }

    async fn load_until(&self, stamp: Timestamp) -> Result<(Vec<ModerationEntry>, bool), String> {
        let mut all: Vec<ModerationEntry> = vec![];
        let mut cursor = ModerationCursor::Latest;
        for _ in 0 .. 10 {
            let page = self.fetch(cursor).await?;
            let full = page.len() as u32 >= MODERATION_PAGE_LIMIT;
            let first = page.first().map(|e| (e.seq, e.received));
            all.splice(0 .. 0, page);
            let Some((first_seq, first_received)) = first else {
                return Ok((all, true));
            };
            if !full {
                return Ok((all, true));
            }
            if first_received <= stamp {
                return Ok((all, false));
            }
            cursor = ModerationCursor::Before(first_seq);
        }
        return Ok((all, false));
    }

    fn create_entries(&self, entries: Vec<ModerationEntry>) -> Vec<Rc<ChatEntry>> {
        self.sync_states(&entries);
        let mut out = vec![];
        for e in entries {
            let message_id = QualifiedMessageId {
                channel: self.0.id.clone(),
                message: e.id.clone(),
            };
            let time = entry_time(&e);
            let blocks = view_blocks(&self.0.id, e.blocks);
            match e.client_id {
                Some(client_id) => {
                    out.push(
                        new_message_copy(
                            &self.0.chat_state,
                            CopyStage::Moderation,
                            (self.0.id.clone(), e.sender.clone(), client_id),
                            Some(message_id),
                            time,
                            blocks,
                        ),
                    );
                },
                None => {
                    out.push(
                        ChatEntry::new_message(
                            self.0.chat_state.clone(),
                            e.sender,
                            Some(message_id),
                            time,
                            Prim::new(blocks),
                            false,
                            None,
                            scope_any(()),
                        ),
                    );
                },
            }
        }
        return out;
    }
}

impl Feed<ChatEntry> for ModerationFeed {
    fn set_parent(&self, parent: WeakInfinite<ChatEntry>) {
        *self.0.parent.borrow_mut() = Some(parent);
    }

    fn request_around(&self, _eg: EventGraph, time: ChatTime) {
        *self.0.pulling_after.borrow_mut() = None;
        *self.0.pulling_before.borrow_mut() = None;
        *self.0.pulling_around.borrow_mut() = Some(spawn_rooted_log("Moderation feed, request around", {
            let self1 = self.clone();
            async move {
                let (all, exhausted) = self1.load_until(time.stamp).await?;
                let (early, late): (Vec<_>, Vec<_>) = all.into_iter().partition(|e| entry_time(e) <= time);
                let early_total = early.len();
                let early = early.into_iter().rev().take(WANT_COUNT).rev().collect::<Vec<_>>();
                let late_total = late.len();
                let late = late.into_iter().take(WANT_COUNT).collect::<Vec<_>>();
                let early_stop = exhausted && early_total <= WANT_COUNT;
                let late_stop = late_total <= WANT_COUNT;
                let mut entries = self1.create_entries(early);
                entries.extend(self1.create_entries(late));
                let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                    return Ok(());
                };
                parent.respond_entries_around(&self1.feed_id(), &time, entries, early_stop, late_stop);
                return Ok(());
            }
        }));
    }

    fn request_before(&self, _eg: EventGraph, time: ChatTime) {
        *self.0.pulling_before.borrow_mut() = Some(spawn_rooted_log("Moderation feed, request before", {
            let self1 = self.clone();
            async move {
                let (entries, stop) = match time.id {
                    ChatTimeId::Moderation(seq) => {
                        let page = self1.fetch(ModerationCursor::Before(seq)).await?;
                        let stop = (page.len() as u32) < MODERATION_PAGE_LIMIT;
                        (page, stop)
                    },
                    _ => {
                        let (all, exhausted) = self1.load_until(time.stamp).await?;
                        let early = all.into_iter().filter(|e| entry_time(e) < time).collect::<Vec<_>>();
                        let total = early.len();
                        let early = early.into_iter().rev().take(WANT_COUNT).rev().collect::<Vec<_>>();
                        (early, exhausted && total <= WANT_COUNT)
                    },
                };
                let entries = self1.create_entries(entries);
                let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                    return Ok(());
                };
                parent.respond_entries_before(&self1.feed_id(), &time, entries, stop);
                return Ok(());
            }
        }));
    }

    fn request_after(&self, _eg: EventGraph, time: ChatTime) {
        *self.0.pulling_after.borrow_mut() = Some(spawn_rooted_log("Moderation feed, request after", {
            let self1 = self.clone();
            async move {
                let (entries, stop) = match time.id {
                    ChatTimeId::Moderation(seq) => {
                        let page = self1.fetch(ModerationCursor::After(seq)).await?;
                        let stop = (page.len() as u32) < MODERATION_PAGE_LIMIT;
                        (page, stop)
                    },
                    _ => {
                        let (all, _) = self1.load_until(time.stamp).await?;
                        let late = all.into_iter().filter(|e| entry_time(e) > time).collect::<Vec<_>>();
                        (late, true)
                    },
                };
                let entries = self1.create_entries(entries);
                let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                    return Ok(());
                };
                parent.respond_entries_after(&self1.feed_id(), &time, entries, stop);
                return Ok(());
            }
        }));
    }
}

pub fn refresh_level(eg: EventGraph, channel: QualifiedChannelId) {
    spawn_log("Refreshing channel level", async move {
        let level =
            req_post_json(&state().env.base_url, c2s::ChannelRefreshLevel { channel: channel.clone() }).await?;
        let mut changed = false;
        eg.event(|pc| {
            if let Some(c) = state().lookup_channel.borrow().get(&channel) {
                if c.level.get() != level {
                    c.level.set(pc, level);
                    changed = true;
                }
            }
        }).unwrap();
        if !changed {
            return Ok(());
        }
        let inf = state().current_chat.borrow().as_ref().and_then(|c| {
            if !c.chat_state2.channel_lookup.borrow().contains_key(&channel) {
                return None;
            }
            return Some(c.inf.clone());
        });
        if let Some(inf) = inf {
            inf.reopen(&ChatFeedId::Moderation(channel));
        }
        return Ok(());
    });
}

pub fn start_level_polling(eg: EventGraph, chat_state2: Rc<ChatState2>) -> ScopeValue {
    let refresh_all = {
        let eg = eg.clone();
        let chat_state2 = chat_state2.clone();
        move || {
            let channels = chat_state2.channel_lookup.borrow().keys().cloned().collect::<Vec<_>>();
            for channel in channels {
                refresh_level(eg.clone(), channel);
            }
        }
    };
    refresh_all();
    let interval = Interval::new(60 * 60 * 1000, {
        let refresh_all = refresh_all.clone();
        move || refresh_all()
    });
    let visibility = EventListener::new(&document(), "visibilitychange", move |_| {
        if document().visibility_state() == VisibilityState::Visible {
            refresh_all();
        }
    });
    return scope_any((interval, visibility));
}
