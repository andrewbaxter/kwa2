use {
    lunk::HistPrim,
    shared::interface::shared::{
        MessageIdem,
        QualifiedChannelId,
        QualifiedMessageId,
    },
    std::{
        cell::RefCell,
        rc::Rc,
    },
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum ChatModeMessage {
    Channel(QualifiedMessageId),
    Outbox(MessageIdem),
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum ChatMode {
    None,
    MessageChannelSelect,
    TopMessage(QualifiedChannelId),
    ReplyMessage(ChatModeMessage),
}

pub struct ChatState {
    pub channels: Rc<RefCell<Vec<(String, QualifiedChannelId)>>>,
    pub mode: HistPrim<ChatMode>,
}
