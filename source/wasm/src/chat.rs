use {
    crate::{
        blocks::ViewBlock,
        chat_entry::{
            ChatEntry,
            ChatEntryMessageInternal,
            ChatTime,
        },
        chat_feed_channel::ChannelFeed,
        chat_feed_moderation::ModerationFeed,
        chat_feed_outbox::OutboxFeed,
        chat_feed_server_outbox::ServerOutboxFeed,
        state::CurrentChatSource,
    },
    by_address::ByAddress,
    defer::defer,
    lunk::{
        EventGraph,
        HistPrim,
        Prim,
    },
    rooting::scope_any,
    shared::interface::shared::{
        MessageClientId,
        ModerationState,
        QualifiedChannelId,
        QualifiedMessageId,
    },
    jiff::Timestamp,
    lunk::ProcessingContext,
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        collections::HashMap,
        hash::Hash,
        rc::{
            Rc,
            Weak,
        },
        str::FromStr,
    },
    wasm_bindgen_futures::spawn_local,
};

pub struct EditOverlay {
    pub stamp: Timestamp,
    pub blocks: Vec<ViewBlock>,
}

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

impl ChatModeReplyMessageTarget {
    pub fn channel(&self) -> &QualifiedChannelId {
        match self {
            ChatModeReplyMessageTarget::Channel(m) => return &m.channel,
            ChatModeReplyMessageTarget::Outbox(m) => return &m.channel,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct ChatModeReplyMessage {
    pub target: ChatModeReplyMessageTarget,
    pub own_identity: Identity,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct ChatModeEditMessage {
    pub target: QualifiedMessageId,
    pub own_identity: Identity,
    pub blocks: ByAddress<Rc<Vec<ViewBlock>>>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum ChatMode {
    None,
    MessageChannelSelect,
    TopMessage(ChatModeTopMessage),
    ReplyMessage(ChatModeReplyMessage),
    EditMessage(ChatModeEditMessage),
}

impl ChatMode {
    pub fn focus(&self) -> Option<ChatModeReplyMessage> {
        match self {
            ChatMode::None | ChatMode::MessageChannelSelect | ChatMode::TopMessage(_) => return None,
            ChatMode::ReplyMessage(m) => return Some(m.clone()),
            ChatMode::EditMessage(m) => return Some(ChatModeReplyMessage {
                target: ChatModeReplyMessageTarget::Channel(m.target.clone()),
                own_identity: m.own_identity.clone(),
            }),
        }
    }
}

pub struct ChannelFeeds {
    pub channel: ChannelFeed,
    pub outboxes: RefCell<HashMap<Identity, OutboxFeed>>,
    pub server_outboxes: RefCell<HashMap<Identity, ServerOutboxFeed>>,
    pub moderation: ModerationFeed,
}

pub type ClientIdKey = (QualifiedChannelId, Identity, MessageClientId);

#[derive(Clone, Copy)]
pub enum CopyStage {
    Device,
    Server,
    Moderation,
    Channel,
}

pub struct ChatState {
    pub eg: EventGraph,
    pub source: CurrentChatSource,
    pub channels_meta: Prim<Vec<(String, Identity, QualifiedChannelId)>>,
    pub entry_channel_lookup: RefCell<HashMap<QualifiedMessageId, Weak<ChatEntry>>>,
    pub entry_channel_lookup_by_client_id: RefCell<HashMap<ClientIdKey, Weak<ChatEntry>>>,
    pub entry_server_outbox_lookup: RefCell<HashMap<ClientIdKey, Weak<ChatEntry>>>,
    pub entry_outbox_lookup: RefCell<HashMap<ClientIdKey, Weak<ChatEntry>>>,
    pub entry_moderation_lookup: RefCell<HashMap<ClientIdKey, Weak<ChatEntry>>>,
    pub edit_overlays: RefCell<HashMap<QualifiedMessageId, EditOverlay>>,
    pub moderation_states: RefCell<HashMap<QualifiedMessageId, Prim<ModerationState>>>,
    pub mode: HistPrim<ChatMode>,
}

impl ChatState {
    pub fn new(
        eg: EventGraph,
        source: CurrentChatSource,
        channels_meta: Vec<(String, Identity, QualifiedChannelId)>,
        mode: HistPrim<ChatMode>,
    ) -> ChatState {
        return ChatState {
            eg: eg,
            source: source,
            channels_meta: Prim::new(channels_meta),
            entry_channel_lookup: Default::default(),
            entry_channel_lookup_by_client_id: Default::default(),
            entry_server_outbox_lookup: Default::default(),
            entry_outbox_lookup: Default::default(),
            entry_moderation_lookup: Default::default(),
            edit_overlays: Default::default(),
            moderation_states: Default::default(),
            mode: mode,
        };
    }

    pub fn overlay_blocks(&self, id: &QualifiedMessageId, blocks: Vec<ViewBlock>) -> Vec<ViewBlock> {
        match self.edit_overlays.borrow().get(id) {
            Some(o) => return o.blocks.clone(),
            None => return blocks,
        }
    }

    pub fn apply_edit_overlay(
        &self,
        pc: &mut ProcessingContext,
        target: QualifiedMessageId,
        client_id: &MessageClientId,
        blocks: Vec<ViewBlock>,
    ) {
        let stamp = Timestamp::from_str(&client_id.0).unwrap();
        {
            let mut overlays = self.edit_overlays.borrow_mut();
            if overlays.get(&target).map(|o| o.stamp >= stamp).unwrap_or(false) {
                return;
            }
            overlays.insert(target.clone(), EditOverlay {
                stamp: stamp,
                blocks: blocks.clone(),
            });
        }
        let Some(e) = self.entry_channel_lookup.borrow().get(&target).and_then(|e| e.upgrade()) else {
            return;
        };
        let internal = e.message().internal.borrow();
        if let ChatEntryMessageInternal::Message(m) = &*internal {
            m.blocks.set(pc, blocks);
        }
    }

    fn stage_lookup(&self, stage: CopyStage) -> &RefCell<HashMap<ClientIdKey, Weak<ChatEntry>>> {
        match stage {
            CopyStage::Device => return &self.entry_outbox_lookup,
            CopyStage::Server => return &self.entry_server_outbox_lookup,
            CopyStage::Moderation => return &self.entry_moderation_lookup,
            CopyStage::Channel => return &self.entry_channel_lookup_by_client_id,
        }
    }

    fn live_copies(&self, key: &ClientIdKey) -> Vec<Rc<ChatEntry>> {
        let mut out = vec![];
        for stage in [CopyStage::Channel, CopyStage::Moderation, CopyStage::Server, CopyStage::Device] {
            if let Some(e) = self.stage_lookup(stage).borrow().get(key).and_then(|e| e.upgrade()) {
                out.push(e);
            }
        }
        return out;
    }

    pub fn find_time_by_client_id(&self, key: &ClientIdKey) -> Option<ChatTime> {
        return self.live_copies(key).into_iter().find(|e| !*e.message().hidden.borrow()).map(|e| e.time.clone());
    }

    pub fn find_time_by_message_id(&self, id: &QualifiedMessageId) -> Option<ChatTime> {
        let e = self.entry_channel_lookup.borrow().get(id).and_then(|e| e.upgrade())?;
        if !*e.message().hidden.borrow() {
            return Some(e.time.clone());
        }
        return self.find_time_by_client_id(e.message().client_key.as_ref().unwrap());
    }
}

pub fn new_message_copy(
    chat_state: &Rc<ChatState>,
    stage: CopyStage,
    key: ClientIdKey,
    message_id: Option<QualifiedMessageId>,
    time: ChatTime,
    blocks: Vec<ViewBlock>,
) -> Rc<ChatEntry> {
    let existing = chat_state.live_copies(&key).into_iter().next();
    let (blocks, hidden) = match existing {
        Some(existing) => {
            let existing_blocks = match &*existing.message().internal.borrow() {
                ChatEntryMessageInternal::Message(m) => Some(m.blocks.clone()),
                ChatEntryMessageInternal::Deleted => None,
            };
            match existing_blocks {
                Some(existing_blocks) => {
                    if let CopyStage::Channel = stage {
                        if *existing_blocks.borrow() != blocks {
                            chat_state.eg.event(|pc| existing_blocks.set(pc, blocks)).unwrap();
                        }
                    }
                    (existing_blocks, true)
                },
                None => (Prim::new(blocks), true),
            }
        },
        None => (Prim::new(blocks), false),
    };
    let entry =
        ChatEntry::new_message(
            chat_state.clone(),
            key.1.clone(),
            message_id.clone(),
            time,
            blocks,
            hidden,
            Some(key.clone()),
            scope_any(defer({
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
            })),
        );
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
