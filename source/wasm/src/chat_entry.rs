use {
    crate::{
        chat::{
            ChatMode,
            ChatState,
        },
        chat_controls::build_chat_entry_controls,
        chat_message::build_chat_entry_message,
        infinite,
    },
    jiff::Timestamp,
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
        },
        wire::c2s::SnapOffset,
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
    Controls,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, PartialOrd, Ord, Hash)]
pub enum ChatTimeId {
    None,
    Outbox(MessageClientId),
    Channel(SnapOffset),
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
    pub body: Prim<String>,
}

#[derive(Clone)]
pub enum ChatEntryMessageInternal {
    // Outbox messages that have since appeared in channel proper
    Obviated,
    Deleted,
    Message(ChatEntryInternalMessage),
}

pub struct ChatEntryMessage {
    pub on_drop: ScopeValue,
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
    pub fn new_message(time: ChatTime, text: String, cleanup: ScopeValue) -> Rc<ChatEntry> {
        let out = Rc::new(ChatEntry {
            time: time,
            int: ChatEntryInternal::Message(ChatEntryMessage {
                on_drop: cleanup,
                internal: Prim::new(
                    ChatEntryMessageInternal::Message(ChatEntryInternalMessage { body: Prim::new(text) }),
                ),
            }),
            el: Default::default(),
        });
        return out;
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
