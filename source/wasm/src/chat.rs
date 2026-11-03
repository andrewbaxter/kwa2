use {
    crate::{
        chat_entry::{
            ChatEntry,
            ChatEntryMessageInternal,
            ChatTime,
        },
        chat_feed_channel::ChannelFeed,
        chat_feed_outbox::OutboxFeed,
        chat_feed_server_outbox::ServerOutboxFeed,
    },
    defer::defer,
    lunk::{
        EventGraph,
        HistPrim,
        Prim,
    },
    rooting::scope_any,
    shared::interface::shared::{
        MessageClientId,
        QualifiedChannelId,
        QualifiedMessageId,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        collections::HashMap,
        hash::Hash,
        rc::{
            Rc,
            Weak,
        },
    },
    wasm_bindgen_futures::spawn_local,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct ChatModeTopMessage {
    pub channel: QualifiedChannelId,
    pub own_identity: Identity,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct ChatModeReplyMessageTargetOutbox {
    pub channel: QualifiedChannelId,
    pub message: MessageClientId,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum ChatModeReplyMessageTarget {
    Channel(QualifiedMessageId),
    Outbox(ChatModeReplyMessageTargetOutbox),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct ChatModeReplyMessage {
    pub target: ChatModeReplyMessageTarget,
    pub own_identity: Identity,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum ChatMode {
    None,
    MessageChannelSelect,
    TopMessage(ChatModeTopMessage),
    ReplyMessage(ChatModeReplyMessage),
}

pub struct ChannelFeeds {
    pub channel: ChannelFeed,
    pub outboxes: RefCell<HashMap<Identity, OutboxFeed>>,
    pub server_outboxes: RefCell<HashMap<Identity, ServerOutboxFeed>>,
}

pub type ClientIdKey = (QualifiedChannelId, Identity, MessageClientId);

#[derive(Clone, Copy)]
pub enum CopyStage {
    Device,
    Server,
    Channel,
}

pub struct ChatState {
    pub eg: EventGraph,
    pub channels_meta: Prim<Vec<(String, Identity, QualifiedChannelId)>>,
    pub entry_channel_lookup: RefCell<HashMap<QualifiedMessageId, Weak<ChatEntry>>>,
    pub entry_channel_lookup_by_client_id: RefCell<HashMap<ClientIdKey, Weak<ChatEntry>>>,
    pub entry_server_outbox_lookup: RefCell<HashMap<ClientIdKey, Weak<ChatEntry>>>,
    pub entry_outbox_lookup: RefCell<HashMap<ClientIdKey, Weak<ChatEntry>>>,
    pub mode: HistPrim<ChatMode>,
}

impl ChatState {
    pub fn new(
        eg: EventGraph,
        channels_meta: Vec<(String, Identity, QualifiedChannelId)>,
        mode: HistPrim<ChatMode>,
    ) -> ChatState {
        return ChatState {
            eg: eg,
            channels_meta: Prim::new(channels_meta),
            entry_channel_lookup: Default::default(),
            entry_channel_lookup_by_client_id: Default::default(),
            entry_server_outbox_lookup: Default::default(),
            entry_outbox_lookup: Default::default(),
            mode: mode,
        };
    }

    fn stage_lookup(&self, stage: CopyStage) -> &RefCell<HashMap<ClientIdKey, Weak<ChatEntry>>> {
        match stage {
            CopyStage::Device => return &self.entry_outbox_lookup,
            CopyStage::Server => return &self.entry_server_outbox_lookup,
            CopyStage::Channel => return &self.entry_channel_lookup_by_client_id,
        }
    }

    fn live_copies(&self, key: &ClientIdKey) -> Vec<Rc<ChatEntry>> {
        let mut out = vec![];
        for stage in [CopyStage::Channel, CopyStage::Server, CopyStage::Device] {
            if let Some(e) = self.stage_lookup(stage).borrow().get(key).and_then(|e| e.upgrade()) {
                out.push(e);
            }
        }
        return out;
    }

    pub fn find_time_by_client_id(&self, key: &ClientIdKey) -> Option<ChatTime> {
        return self
            .live_copies(key)
            .into_iter()
            .find(|e| !*e.message().hidden.borrow())
            .map(|e| e.time.clone());
    }

    pub fn find_time_by_message_id(&self, id: &QualifiedMessageId) -> Option<ChatTime> {
        let e = self.entry_channel_lookup.borrow().get(id).and_then(|e| e.upgrade())?;
        if !*e.message().hidden.borrow() {
            return Some(e.time.clone());
        }
        return self.find_time_by_client_id(e.message().client_key.as_ref().unwrap());
    }
}

fn reveal_if_none_visible(chat_state: Weak<ChatState>, key: ClientIdKey) {
    spawn_local(async move {
        let Some(chat_state) = chat_state.upgrade() else {
            return;
        };
        chat_state.eg.event(|pc| {
            let copies = chat_state.live_copies(&key);
            if copies.iter().any(|e| !*e.message().hidden.borrow()) {
                return;
            }
            if let Some(e) = copies.first() {
                e.message().hidden.set(pc, false);
            }
        }).unwrap();
    });
}

pub fn new_message_copy(
    chat_state: &Rc<ChatState>,
    stage: CopyStage,
    key: ClientIdKey,
    message_id: Option<QualifiedMessageId>,
    time: ChatTime,
    body: String,
) -> Rc<ChatEntry> {
    let existing = chat_state.live_copies(&key).into_iter().next();
    let (body, hidden) = match existing {
        Some(existing) => {
            let existing_body = match &*existing.message().internal.borrow() {
                ChatEntryMessageInternal::Message(m) => Some(m.body.clone()),
                ChatEntryMessageInternal::Deleted => None,
            };
            match existing_body {
                Some(existing_body) => {
                    if let CopyStage::Channel = stage {
                        if *existing_body.borrow() != body {
                            chat_state.eg.event(|pc| existing_body.set(pc, body)).unwrap();
                        }
                    }
                    (existing_body, true)
                },
                None => (Prim::new(body), true),
            }
        },
        None => (Prim::new(body), false),
    };
    let entry = ChatEntry::new_message(chat_state.clone(), key.1.clone(), message_id.clone(), time, body, hidden, Some(key.clone()), scope_any(defer({
        let chat_state = Rc::downgrade(chat_state);
        let key = key.clone();
        let message_id = message_id.clone();
        move || {
            let Some(chat_state1) = chat_state.upgrade() else {
                return;
            };
            remove_dead_entry(chat_state1.stage_lookup(stage), &key);
            if let Some(message_id) = &message_id {
                remove_dead_entry(&chat_state1.entry_channel_lookup, message_id);
            }
            reveal_if_none_visible(chat_state, key);
        }
    })));
    chat_state.stage_lookup(stage).borrow_mut().insert(key, Rc::downgrade(&entry));
    if let Some(message_id) = message_id {
        chat_state.entry_channel_lookup.borrow_mut().insert(message_id, Rc::downgrade(&entry));
    }
    return entry;
}

pub fn remove_dead_entry<K: Eq + Hash>(map: &RefCell<HashMap<K, Weak<ChatEntry>>>, key: &K) {
    let mut map = map.borrow_mut();
    if map.get(key).map(|e| e.strong_count() == 0).unwrap_or(false) {
        map.remove(key);
    }
}

pub struct ChatState2 {
    pub channel_lookup: RefCell<HashMap<QualifiedChannelId, ChannelFeeds>>,
}
