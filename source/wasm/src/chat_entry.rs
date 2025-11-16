use {
    crate::{
        chat_controls::build_chat_entry_controls,
        chat_message::build_chat_entry_message,
        infinite,
    },
    defer::defer,
    jiff::Timestamp,
    lunk::{
        HistPrim,
        Prim,
    },
    rooting::{
        scope_any,
        ScopeValue,
        WeakEl,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::{
        shared::{
            MessageIdem,
            QualifiedChannelId,
            QualifiedMessageId,
        },
        wire::c2s::SnapOffset,
    },
    std::{
        cell::RefCell,
        collections::HashMap,
        hash::Hash,
        rc::Rc,
    },
};

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, PartialOrd, Ord, Hash)]
pub enum ChatFeedId {
    Channel(QualifiedChannelId),
    Outbox(QualifiedChannelId),
    Controls,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, PartialOrd, Ord, Hash)]
pub enum ChatTimeId {
    None,
    Outbox(MessageIdem),
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

type ChatEntryLookup_<K> = RefCell<HashMap<K, Rc<ChatEntry>>>;

pub struct ChatEntryLookup<K: Eq + Hash>(pub Rc<ChatEntryLookup_<K>>);

impl<K: Eq + Hash> Clone for ChatEntryLookup<K> {
    fn clone(&self) -> Self {
        return Self(self.0.clone());
    }
}

impl<K: Eq + Hash> ChatEntryLookup<K> {
    pub fn new() -> Self {
        return Self(Rc::new(RefCell::new(HashMap::new())));
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
    pub group_mode: HistPrim<ChatMode>,
    pub channels: Rc<RefCell<Vec<(String, QualifiedChannelId)>>>,
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
    pub fn new_message<
        K: 'static + Eq + Hash + Clone,
    >(map: &ChatEntryLookup<K>, time: ChatTime, lookup_id: K, text: String) -> Rc<ChatEntry> {
        let out = Rc::new(ChatEntry {
            time: time,
            int: ChatEntryInternal::Message(ChatEntryMessage {
                on_drop: scope_any(defer({
                    let map = Rc::downgrade(&map.0);
                    let lookup_id = lookup_id.clone();
                    move || {
                        let Some(map) = map.upgrade() else {
                            return;
                        };
                        map.borrow_mut().remove(&lookup_id);
                    }
                })),
                internal: Prim::new(
                    ChatEntryMessageInternal::Message(ChatEntryInternalMessage { body: Prim::new(text) }),
                ),
            }),
            el: Default::default(),
        });
        map.0.borrow_mut().insert(lookup_id, out.clone());
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
