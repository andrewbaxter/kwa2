use {
    crate::{
        blocks::ViewBlock,
        chat::{
            ChatMode,
            ChatState,
            ClientIdKey,
        },
        chat_controls::build_chat_entry_controls,
        chat_message::build_chat_entry_message,
        infinite,
    },
    flowcontrol::exenum,
    jiff::{
        Timestamp,
    },
    lunk::{
        HistPrim,
        Prim,
    },
    rooting::{
        ScopeValue,
        WeakEl,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::{
        shared::{
            MessageClientId,
            QualifiedChannelId,
            QualifiedMessageId,
        },
        wire::c2s::SnapPageOffsetPos,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        hash::Hash,
        rc::Rc,
    },
};

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, PartialOrd, Ord, Hash)]
pub enum ChatFeedId {
    Channel(QualifiedChannelId),
    Outbox((Identity, QualifiedChannelId)),
    ServerOutbox((Identity, QualifiedChannelId)),
    Moderation(QualifiedChannelId),
    Controls,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, PartialOrd, Ord, Hash)]
pub enum ChatTimeId {
    None,
    Outbox(MessageClientId),
    ServerOutbox(u64),
    Moderation(u64),
    Channel(SnapPageOffsetPos),
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, PartialOrd, Ord, Hash)]
pub struct ChatTime {
    pub stamp: Timestamp,
    pub id: ChatTimeId,
}

impl Default for ChatTime {
    fn default() -> Self {
        return Self {
            stamp: Timestamp::now(),
            id: ChatTimeId::None,
        };
    }
}

#[derive(Clone)]
pub struct ChatEntryInternalMessage {
    pub time: Timestamp,
    pub blocks: Prim<Vec<ViewBlock>>,
}

#[derive(Clone)]
pub enum ChatEntryMessageInternal {
    Deleted,
    Message(ChatEntryInternalMessage),
}

pub struct ChatEntryMessage {
    pub on_drop: ScopeValue,
    pub chat_state: Rc<ChatState>,
    pub sender: Identity,
    pub message_id: Option<QualifiedMessageId>,
    pub client_key: Option<ClientIdKey>,
    pub hidden: Prim<bool>,
    pub internal: Prim<ChatEntryMessageInternal>,
}

pub struct ChatEntryControls {
    pub mode: HistPrim<ChatMode>,
    pub state: Rc<ChatState>,
}

pub enum ChatEntryInternal {
    Controls(ChatEntryControls),
    Message(ChatEntryMessage),
}

pub struct ChatEntry {
    pub time: ChatTime,
    pub int: ChatEntryInternal,
    pub el: RefCell<Option<WeakEl>>,
}

impl ChatEntry {
    pub fn new_message(
        chat_state: Rc<ChatState>,
        sender: Identity,
        message_id: Option<QualifiedMessageId>,
        time: ChatTime,
        blocks: Prim<Vec<ViewBlock>>,
        hidden: bool,
        client_key: Option<ClientIdKey>,
        cleanup: ScopeValue,
    ) -> Rc<ChatEntry> {
        return Rc::new(ChatEntry {
            time: time.clone(),
            int: ChatEntryInternal::Message(ChatEntryMessage {
                on_drop: cleanup,
                chat_state: chat_state,
                sender: sender,
                message_id: message_id,
                client_key: client_key,
                hidden: Prim::new(hidden),
                internal: Prim::new(ChatEntryMessageInternal::Message(ChatEntryInternalMessage {
                    time: time.stamp,
                    blocks: blocks,
                })),
            }),
            el: Default::default(),
        });
    }

    pub fn message(&self) -> &ChatEntryMessage {
        return exenum!(&self.int, ChatEntryInternal:: Message(e) => e).unwrap();
    }
}

impl infinite::Entry for ChatEntry {
    type FeedId = ChatFeedId;
    type Time = ChatTime;

    fn create_el(&self, pc: &mut lunk::ProcessingContext) -> rooting::El {
        let mut e = self.el.borrow_mut();
        if let Some(e) = e.as_ref().and_then(|x| x.upgrade()) {
            return e.clone();
        };
        let out = match &self.int {
            ChatEntryInternal::Controls(m) => build_chat_entry_controls(pc, m),
            ChatEntryInternal::Message(m) => build_chat_entry_message(pc, m),
        };
        *e = Some(out.weak());
        return out;
    }

    fn time(&self) -> Self::Time {
        return self.time.clone();
    }
}
