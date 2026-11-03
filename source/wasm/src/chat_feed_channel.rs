use {
    crate::{
        api::{
            req_get_retry,
        },
        chat::{
            ChatMode,
            ChatModeReplyMessage,
            ChatModeReplyMessageTarget,
            ChatState,
            CopyStage,
            new_message_copy,
            remove_dead_entry,
        },
        chat_entry::{
            ChatEntry,
            ChatEntryMessageInternal,
            ChatFeedId,
            ChatTime,
            ChatTimeId,
        },
        infinite::{
            Feed,
            WeakInfinite,
        },
        outbox::opfs_delete_outbox_message,
        state::{
            spawn_rooted_log,
            state,
        },
    },
    defer::defer,
    flowcontrol::exenum,
    futures::channel::oneshot,
    lunk::{
        EventGraph,
        Prim,
    },
    rooting::scope_any,
    shared::interface::{
        shared::{
            MessageBody,
            MessageRel,
            QualifiedChannelId,
            QualifiedMessageId,
        },
        wire::c2s::{
            self,
            ActivityOffset,
            ActivityPage,
            ActivityPageOffsetPos,
            PagePosition,
            SnapMessage,
            SnapPage,
            SnapPageOffsetPos,
        },
    },
    std::{
        cell::{
            Cell,
            RefCell,
        },
        rc::Rc,
    },
};

pub struct ChannelFeed_ {
    id: QualifiedChannelId,
    chat_state: Rc<ChatState>,
    parent: RefCell<Option<WeakInfinite<ChatEntry>>>,
    /// Ignore server-push for events older than we've already observed
    known_latest_server_time: Cell<Option<ActivityOffset>>,
    have_data_time: Cell<Option<ActivityPageOffsetPos>>,
    pulling_snap_around: RefCell<Option<oneshot::Receiver<()>>>,
    pulling_snap_before: RefCell<Option<oneshot::Receiver<()>>>,
    pulling_snap_after: RefCell<Option<oneshot::Receiver<()>>>,
    pulling_activity: RefCell<Option<oneshot::Receiver<()>>>,
}

#[derive(Clone)]
pub struct ChannelFeed(Rc<ChannelFeed_>);

impl ChannelFeed {
    pub fn new(chat_state: Rc<ChatState>, id: QualifiedChannelId) -> Self {
        return ChannelFeed(Rc::new(ChannelFeed_ {
            chat_state: chat_state,
            id: id,
            parent: RefCell::new(None),
            known_latest_server_time: Cell::new(None),
            have_data_time: Cell::new(None),
            pulling_snap_around: RefCell::new(None),
            pulling_snap_before: RefCell::new(None),
            pulling_snap_after: RefCell::new(None),
            pulling_activity: RefCell::new(None),
        }));
    }

    pub fn notify(&self, eg: &EventGraph, offset: ActivityOffset) {
        self.update_known_time(eg, offset);
    }

    fn update_known_or_have_time(&self, eg: &EventGraph, new_data_time: ActivityPageOffsetPos) {
        if self
            .0
            .have_data_time
            .get()
            .map(|data_time| new_data_time.offset_pos < data_time.offset_pos)
            .unwrap_or(true) {
            // If empty or resp time older, update
            self.0.have_data_time.set(Some(new_data_time));
        }
        self.update_known_time(eg, new_data_time.offset_pos.offset);
    }

    fn update_known_time(&self, eg: &EventGraph, new_offset: ActivityOffset) {
        if self.0.known_latest_server_time.get().map(|known_time| new_offset > known_time).unwrap_or(true) {
            // If empty or resp time newer, update
            self.0.known_latest_server_time.set(Some(new_offset));
        }
        if self.0.known_latest_server_time.get() != self.0.have_data_time.get().map(|x| x.offset_pos.offset) {
            *self.0.pulling_activity.borrow_mut() = Some(spawn_rooted_log("pulling new channel events", {
                let self1 = self.clone();
                let eg = eg.clone();
                async move {
                    let mut saw_new = false;
                    loop {
                        let have_data_time = self1.0.have_data_time.get();
                        let Some(known_server_time) = self1.0.known_latest_server_time.get() else {
                            break;
                        };
                        if let Some(have_data_time) = &have_data_time {
                            if have_data_time.offset_pos.offset >= known_server_time {
                                self1.0.known_latest_server_time.set(Some(have_data_time.offset_pos.offset));
                                break;
                            }
                        }
                        let next_page = match &have_data_time {
                            Some(have_data_time) => match &have_data_time.offset_pos.pos {
                                PagePosition::Last => ActivityPage(have_data_time.page.0 + 1),
                                _ => have_data_time.page,
                            },
                            None => ActivityPage(0),
                        };
                        let Some(resp) = req_get_retry(c2s::GetActivityPage {
                            channel: self1.0.id.clone(),
                            page: next_page,
                        }).await? else {
                            break;
                        };
                        let mut server_time = None;
                        eg.event(|pc| {
                            for entry in resp.messages.into_iter() {
                                if let Some(have_data_time) = &have_data_time {
                                    if next_page == have_data_time.page &&
                                        entry.offset_pos.offset <= have_data_time.offset_pos.offset {
                                        continue;
                                    }
                                }
                                server_time = Some(ActivityPageOffsetPos {
                                    page: next_page,
                                    offset_pos: entry.offset_pos,
                                });
                                let entry1 = match serde_json::from_str::<MessageBody>(&entry.message.0.message) {
                                    Ok(e) => e,
                                    Err(e) => {
                                        state()
                                            .log
                                            .log(
                                                &format!("Error deserializing message from activity, skipping: {}", e),
                                            );
                                        continue;
                                    },
                                };
                                let entries = self1.0.chat_state.entry_channel_lookup.borrow();
                                match entry1.rel {
                                    MessageRel::EditOf(target) => {
                                        let Some(e) = entries.get(&QualifiedMessageId {
                                            channel: self1.0.id.clone(),
                                            message: target,
                                        }).and_then(|e| e.upgrade()) else {
                                            continue;
                                        };
                                        let e_int = e.message().internal.borrow();
                                        match &*e_int {
                                            ChatEntryMessageInternal::Deleted => { },
                                            ChatEntryMessageInternal::Message(m) => {
                                                m.body.set(pc, entry1.body.clone());
                                            },
                                        }
                                    },
                                    MessageRel::None | MessageRel::ReplyTo(_) => {
                                        if !entries.contains_key(&QualifiedMessageId {
                                            channel: self1.0.id.clone(),
                                            message: entry1.id,
                                        }) {
                                            saw_new = true;
                                        }
                                    },
                                }
                            }
                        });
                        let Some(server_time) = server_time else {
                            break;
                        };
                        self1.0.have_data_time.set(Some(server_time));
                    }
                    if saw_new {
                        let parent = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade());
                        if let Some(parent) = parent {
                            if let Some(pivot) = parent.want_after(ChatFeedId::Channel(self1.0.id.clone())) {
                                self1.request_after(eg.clone(), pivot);
                            }
                        }
                    }
                    return Ok(());
                }
            }));
        }
    }
}

async fn create_entries(
    eg: &EventGraph,
    chat_state: &Rc<ChatState>,
    snap_page: SnapPage,
    messages: Vec<SnapMessage>,
) -> Vec<Rc<ChatEntry>> {
    let mut out = vec![];
    let mut arrived = vec![];
    for e in messages {
        let time = ChatTime {
            stamp: e.original_receive_time,
            id: ChatTimeId::Channel(SnapPageOffsetPos {
                page: snap_page.clone(),
                offset_pos: e.offset_pos,
            }),
        };
        match e.client_id {
            Some(client_id) => {
                out.push(
                    new_message_copy(
                        chat_state,
                        CopyStage::Channel,
                        (e.original_id.channel.clone(), e.original_id.message.identity.clone(), client_id.clone()),
                        Some(e.original_id.clone()),
                        time,
                        e.message.body,
                    ),
                );
                arrived.push((client_id, e.original_id));
            },
            None => {
                let entry =
                    ChatEntry::new_message(
                        chat_state.clone(),
                        e.original_id.message.identity.clone(),
                        Some(e.original_id.clone()),
                        time,
                        Prim::new(e.message.body),
                        false,
                        None,
                        scope_any(defer({
                            let chat_state = Rc::downgrade(chat_state);
                            let id = e.original_id.clone();
                            move || {
                                let Some(chat_state) = chat_state.upgrade() else {
                                    return;
                                };
                                remove_dead_entry(&chat_state.entry_channel_lookup, &id);
                            }
                        })),
                    );
                chat_state.entry_channel_lookup.borrow_mut().insert(e.original_id, Rc::downgrade(&entry));
                out.push(entry);
            },
        }
    }
    eg.event(|pc| {
        for (client_id, id) in &arrived {
            let own_ident = {
                let mode = chat_state.mode.borrow();
                let ChatMode::ReplyMessage(mode) = &*mode else {
                    continue;
                };
                let ChatModeReplyMessageTarget::Outbox(target) = &mode.target else {
                    continue;
                };
                if target.channel != id.channel || target.message != *client_id ||
                    mode.own_identity != id.message.identity {
                    continue;
                }
                mode.own_identity.clone()
            };
            chat_state.mode.set(pc, ChatMode::ReplyMessage(ChatModeReplyMessage {
                target: ChatModeReplyMessageTarget::Channel(id.clone()),
                own_identity: own_ident,
            }));
        }
    }).unwrap();
    for (client_id, id) in &arrived {
        opfs_delete_outbox_message(&id.message.identity, &id.channel, client_id).await;
    }
    return out;
}

impl Feed<ChatEntry> for ChannelFeed {
    fn set_parent(&self, parent: WeakInfinite<ChatEntry>) {
        *self.0.parent.borrow_mut() = Some(parent);
    }

    fn request_around(&self, eg: EventGraph, time: ChatTime) {
        *self.0.pulling_snap_after.borrow_mut() = None;
        *self.0.pulling_snap_before.borrow_mut() = None;
        *self.0.pulling_activity.borrow_mut() = None;
        *self.0.pulling_snap_around.borrow_mut() = Some(spawn_rooted_log("Channel feed - requesting messages around", {
            let self1 = self.clone();
            async move {
                let page = match req_get_retry(c2s::SnapPageContainingTime {
                    channel: self1.0.id.clone(),
                    time: time.stamp.clone(),
                }).await? {
                    Some(page) => page,
                    None => SnapPage(0),
                };
                let Some(resp) = req_get_retry(c2s::GetSnapPage {
                    channel: self1.0.id.clone(),
                    page: page,
                }).await? else {
                    // Just expired? Shouldn't happen generally
                    let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                        return Ok(());
                    };
                    parent.respond_entries_around(
                        &ChatFeedId::Channel(self1.0.id.clone()),
                        &time,
                        vec![],
                        true,
                        true,
                    );
                    return Ok(());
                };
                {
                    let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                        return Ok(());
                    };
                    let late_stop =
                        resp.messages.last().map(|v| v.offset_pos.pos != PagePosition::Last).unwrap_or(true);
                    parent.respond_entries_around(
                        &ChatFeedId::Channel(self1.0.id.clone()),
                        &time,
                        create_entries(&eg, &self1.0.chat_state, page, resp.messages).await,
                        page.0 == 0,
                        late_stop,
                    );
                }
                if let Some(latest_activity) = resp.latest_activity {
                    self1.update_known_or_have_time(&eg, latest_activity);
                }
                return Ok(());
            }
        }));
    }

    fn request_before(&self, eg: EventGraph, time: ChatTime) {
        *self.0.pulling_snap_before.borrow_mut() = Some(spawn_rooted_log("Channel feed, requesting messages before", {
            let self1 = self.clone();
            async move {
                let offset = exenum!(time.id, ChatTimeId:: Channel(c) => c).unwrap();
                if offset.offset_pos.offset.0 == 0 {
                    let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                        return Ok(());
                    };
                    parent.respond_entries_before(&ChatFeedId::Channel(self1.0.id.clone()), &time, vec![], true);
                    return Ok(());
                }
                let page = match &offset.offset_pos.pos {
                    PagePosition::First => SnapPage(offset.page.0 - 1),
                    _ => offset.page,
                };
                let Some(mut resp): Option<c2s::SnapPageRes> = req_get_retry(c2s::GetSnapPage {
                    channel: self1.0.id.clone(),
                    page: page,
                }).await? else {
                    let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                        return Ok(());
                    };
                    parent.respond_entries_before(&ChatFeedId::Channel(self1.0.id.clone()), &time, vec![], true);
                    return Ok(());
                };
                resp.messages.retain(|v| v.offset_pos.offset < offset.offset_pos.offset);
                resp.messages.reverse();
                {
                    let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                        return Ok(());
                    };
                    parent.respond_entries_before(
                        &ChatFeedId::Channel(self1.0.id.clone()),
                        &time,
                        create_entries(&eg, &self1.0.chat_state, page, resp.messages).await,
                        page.0 == 0,
                    );
                }
                if let Some(latest_activity) = resp.latest_activity {
                    self1.update_known_or_have_time(&eg, latest_activity);
                }
                return Ok(());
            }
        }));
    }

    fn request_after(&self, eg: EventGraph, time: ChatTime) {
        *self.0.pulling_snap_after.borrow_mut() = Some(spawn_rooted_log("Channel feed, requesting messages after", {
            let self1 = self.clone();
            async move {
                let (page, min_offset) = match &time.id {
                    ChatTimeId::Channel(offset) => (match &offset.offset_pos.pos {
                        PagePosition::Last => SnapPage(offset.page.0 + 1),
                        _ => offset.page,
                    }, Some(offset.offset_pos.offset)),
                    ChatTimeId::None | ChatTimeId::Outbox(_) | ChatTimeId::ServerOutbox(_) => (SnapPage(0), None),
                };
                let Some(mut resp): Option<c2s::SnapPageRes> = req_get_retry(c2s::GetSnapPage {
                    channel: self1.0.id.clone(),
                    page: page,
                }).await? else {
                    let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                        return Ok(());
                    };
                    parent.respond_entries_after(&ChatFeedId::Channel(self1.0.id.clone()), &time, vec![], true);
                    return Ok(());
                };
                let stop = resp.messages.last().map(|v| v.offset_pos.pos != PagePosition::Last).unwrap_or(true);
                if let Some(min_offset) = min_offset {
                    resp.messages.retain(|v| v.offset_pos.offset > min_offset);
                }
                {
                    let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                        return Ok(());
                    };
                    parent.respond_entries_after(
                        &ChatFeedId::Channel(self1.0.id.clone()),
                        &time,
                        create_entries(&eg, &self1.0.chat_state, page, resp.messages).await,
                        stop,
                    );
                }
                if let Some(latest_activity) = resp.latest_activity {
                    self1.update_known_or_have_time(&eg, latest_activity);
                }
                return Ok(());
            }
        }));
    }
}
