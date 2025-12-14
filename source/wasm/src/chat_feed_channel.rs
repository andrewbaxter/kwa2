use {
    crate::{
        api::req_get,
        chat::{
            ChatMode,
            ChatModeReplyMessage,
            ChatModeReplyMessageTarget,
            ChatState,
        },
        chat_entry::{
            ChatEntry,
            ChatEntryInternal,
            ChatEntryMessageInternal,
            ChatFeedId,
            ChatTime,
            ChatTimeId,
        },
        infinite::{
            Feed,
            WeakInfinite,
        },
        opfs::{
            opfs_list_dir,
            opfs_read_json,
        },
        outbox::{
            OPFS_FILENAME_MAIN,
            OutboxMessage,
            opfs_outbox_channel_dir,
        },
        state::{
            spawn_rooted_log,
            state,
        },
    },
    defer::defer,
    flowcontrol::{
        exenum,
        shed,
    },
    futures::channel::oneshot,
    lunk::EventGraph,
    rooting::scope_any,
    shared::interface::{
        shared::{
            MessageClientId,
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
    wasm_bindgen::JsValue,
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        FileSystemDirectoryHandle,
        console,
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

    fn maybe_trigger_pull(&self, eg: &EventGraph) {
        if self.0.known_latest_server_time.get() != self.0.have_data_time.get().map(|x| x.offset_pos.offset) {
            *self.0.pulling_activity.borrow_mut() = Some(spawn_rooted_log("pulling new channel events", {
                let self1 = self.clone();
                let eg = eg.clone();
                async move {
                    loop {
                        let Some(have_data_time) = self1.0.have_data_time.get() else {
                            break;
                        };
                        let known_server_time = self1.0.known_latest_server_time.get().unwrap();
                        if have_data_time.offset_pos.offset >= known_server_time {
                            self1.0.known_latest_server_time.set(Some(have_data_time.offset_pos.offset));
                            break;
                        }
                        let next_page = match &have_data_time.offset_pos.pos {
                            PagePosition::First => ActivityPage(have_data_time.page.0 - 1),
                            _ => have_data_time.page,
                        };
                        let Some(resp) = req_get(&state().env.base_url, c2s::GetActivityPage {
                            channel: self1.0.id.clone(),
                            page: next_page,
                        }).await? else {
                            break;
                        };
                        let mut server_time = None;
                        eg.event(|pc| {
                            for entry in resp.messages.into_iter() {
                                let entry1 = match entry.message.0.get_no_verify() {
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
                                server_time = Some(ActivityPageOffsetPos {
                                    page: next_page,
                                    offset_pos: entry.offset_pos,
                                });
                                let entries = self1.0.chat_state.entry_channel_lookup.borrow_mut();
                                let Some(e) = entries.get(&QualifiedMessageId {
                                    channel: self1.0.id.clone(),
                                    message: entry1.id,
                                }) else {
                                    continue;
                                };
                                let e = exenum!(&e.int, ChatEntryInternal:: Message(e) => e).unwrap();
                                let e_int = e.internal.borrow();
                                match &*e_int {
                                    ChatEntryMessageInternal::Obviated => { },
                                    ChatEntryMessageInternal::Deleted => { },
                                    ChatEntryMessageInternal::Message(m) => {
                                        m.body.set(pc, entry1.body.clone());
                                    },
                                }
                            }
                        });
                        self1.0.have_data_time.set(server_time);
                    }
                    return Ok(());
                }
            }));
        }
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
        self.maybe_trigger_pull(eg);
    }
}

async fn obviate_outbox_entry(
    eg: &EventGraph,
    chat_state: &Rc<ChatState>,
    channel: &QualifiedChannelId,
    idem: &MessageClientId,
    id: &QualifiedMessageId,
) {
    eg.event(|pc| {
        if let Some(e) =
            chat_state
                .entry_outbox_lookup
                .borrow()
                .get(&(channel.clone(), id.message.identity.clone(), idem.clone())) {
            let e = exenum!(&e.int, ChatEntryInternal:: Message(e) => e).unwrap();
            e.internal.set(pc, ChatEntryMessageInternal::Obviated);
        }
        shed!{
            let mode = chat_state.mode.borrow();
            let ChatMode::ReplyMessage(mode) = &*mode else {
                break;
            };
            let ChatModeReplyMessageTarget::Outbox(target) = &mode.target else {
                break;
            };
            if target.message != *idem {
                break;
            }
            let own_ident = mode.own_identity.clone();
            chat_state.mode.set(pc, ChatMode::ReplyMessage(ChatModeReplyMessage {
                target: ChatModeReplyMessageTarget::Channel(id.clone()),
                own_identity: own_ident,
            }));
        }
    }).unwrap();
    let channel_dir = opfs_outbox_channel_dir(&id.message.identity, channel).await;
    for (k, v) in opfs_list_dir(&channel_dir).await {
        let v = FileSystemDirectoryHandle::from(v);
        let m = match opfs_read_json::<OutboxMessage>(&v, OPFS_FILENAME_MAIN).await {
            Ok(v) => v,
            Err(e) => {
                console::log_1(
                    &JsValue::from(
                        format!("Couldn't open [{}] file for outbox entry {}: {}", OPFS_FILENAME_MAIN, k, e),
                    ),
                );
                continue;
            },
        };
        if m.client_id != *idem {
            continue;
        }
        if let Err(e) = JsFuture::from(channel_dir.remove_entry(&k)).await {
            console::log_2(&JsValue::from("Error removing matched outbox entry"), &e);
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
    for e in messages {
        if let Some(client_id) = &e.client_id {
            obviate_outbox_entry(eg, chat_state, &e.original_id.channel, client_id, &e.original_id).await;
        }
        let entry = ChatEntry::new_message(ChatTime {
            stamp: e.original_receive_time,
            id: ChatTimeId::Channel(SnapPageOffsetPos {
                page: snap_page.clone(),
                offset_pos: e.offset_pos,
            }),
        }, e.message.body, scope_any(defer({
            let chat_state = Rc::downgrade(chat_state);
            let id = e.original_id.clone();
            let client_id = e.client_id.clone();
            move || {
                let Some(chat_state) = chat_state.upgrade() else {
                    return;
                };
                chat_state.entry_channel_lookup.borrow_mut().remove(&id);
                if let Some(client_id) = &client_id {
                    chat_state
                        .entry_channel_lookup_by_client_id
                        .borrow_mut()
                        .remove(&(id.channel.clone(), id.channel.identity.clone(), client_id.clone()));
                }
            }
        })));
        out.push(entry.clone());
        chat_state.entry_channel_lookup.borrow_mut().insert(e.original_id.clone(), entry.clone());
        if let Some(client_id) = &e.client_id {
            chat_state
                .entry_channel_lookup_by_client_id
                .borrow_mut()
                .insert(
                    (e.original_id.channel.clone(), e.original_id.channel.identity.clone(), client_id.clone()),
                    entry.clone(),
                );
        }
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
                let Some(page) = req_get(&state().env.base_url, c2s::SnapPageContainingTime {
                    channel: self1.0.id.clone(),
                    time: time.stamp.clone(),
                }).await? else {
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
                let Some(resp) = req_get(&state().env.base_url, c2s::GetSnapPage {
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
                    parent.respond_entries_around(
                        &ChatFeedId::Channel(self1.0.id.clone()),
                        &time,
                        create_entries(&eg, &self1.0.chat_state, page, resp.messages).await,
                        page.0 == 0,
                        false,
                    );
                }
                self1.update_known_or_have_time(&eg, resp.latest_activity);
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
                let Some(mut resp): Option<c2s::SnapPageRes> = req_get(&state().env.base_url, c2s::GetSnapPage {
                    channel: self1.0.id.clone(),
                    page: page,
                }).await? else {
                    return Ok(());
                };
                resp
                    .messages
                    .splice(
                        resp
                            .messages
                            .iter()
                            .enumerate()
                            .find_map(|(i, v)| if v.offset_pos.offset >= offset.offset_pos.offset {
                                Some(i)
                            } else {
                                None
                            })
                            .unwrap_or(resp.messages.len()) .. resp.messages.len(),
                        [],
                    );
                {
                    let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                        return Ok(());
                    };
                    parent.respond_entries_before(
                        &ChatFeedId::Channel(self1.0.id.clone()),
                        &time,
                        create_entries(&eg, &self1.0.chat_state, page, resp.messages).await,
                        false,
                    );
                }
                self1.update_known_or_have_time(&eg, resp.latest_activity);
                return Ok(());
            }
        }));
    }

    fn request_after(&self, eg: EventGraph, time: ChatTime) {
        *self.0.pulling_snap_after.borrow_mut() = Some(spawn_rooted_log("Channel feed, requesting messages after", {
            let self1 = self.clone();
            async move {
                let offset = exenum!(time.id, ChatTimeId:: Channel(c) => c).unwrap();
                let page = match &offset.offset_pos.pos {
                    PagePosition::Last => SnapPage(offset.page.0 + 1),
                    _ => offset.page,
                };
                let Some(mut resp): Option<c2s::SnapPageRes> = req_get(&state().env.base_url, c2s::GetSnapPage {
                    channel: self1.0.id.clone(),
                    page: page,
                }).await? else {
                    let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                        return Ok(());
                    };
                    parent.respond_entries_after(&ChatFeedId::Channel(self1.0.id.clone()), &time, vec![], true);
                    return Ok(());
                };
                resp
                    .messages
                    .splice(
                        0 ..
                            resp
                                .messages
                                .iter()
                                .enumerate()
                                .filter(|(_, v)| v.offset_pos.offset <= offset.offset_pos.offset)
                                .map(|x| x.0)
                                .last()
                                .unwrap_or(0),
                        [],
                    );
                {
                    let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                        return Ok(());
                    };
                    parent.respond_entries_after(
                        &ChatFeedId::Channel(self1.0.id.clone()),
                        &time,
                        create_entries(&eg, &self1.0.chat_state, page, resp.messages).await,
                        false,
                    );
                }
                self1.update_known_or_have_time(&eg, resp.latest_activity);
                return Ok(());
            }
        }));
    }
}
