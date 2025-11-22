use {
    crate::{
        chat_entry::ChatEntry,
        chat_feed_channel::ChannelFeed,
        chat_feed_outbox::OutboxFeed,
    },
    lunk::HistPrim,
    shared::interface::shared::{
        MessageClientId,
        QualifiedChannelId,
        QualifiedMessageId,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
    },
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
}

pub struct ChatState {
    pub channels_meta: RefCell<Vec<(String, Identity, QualifiedChannelId)>>,
    pub entry_channel_lookup: RefCell<HashMap<QualifiedMessageId, Rc<ChatEntry>>>,
    pub entry_channel_lookup_by_client_id: RefCell<
        HashMap<(QualifiedChannelId, Identity, MessageClientId), Rc<ChatEntry>>,
    >,
    pub entry_outbox_lookup: RefCell<HashMap<(QualifiedChannelId, Identity, MessageClientId), Rc<ChatEntry>>>,
    pub mode: HistPrim<ChatMode>,
}

pub struct ChatState2 {
    pub channel_lookup: RefCell<HashMap<QualifiedChannelId, ChannelFeeds>>,
}
