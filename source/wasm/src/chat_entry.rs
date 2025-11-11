use {
    crate::infinite,
    defer::defer,
    jiff::Timestamp,
    rooting::{
        el,
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
            OutboxMessageId,
            QualifiedChannelId,
        },
        wire::c2s::SnapOffset,
    },
    std::{
        cell::RefCell,
        collections::HashMap,
        hash::Hash,
        rc::{
            Rc,
        },
    },
};

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, PartialOrd, Ord, Hash)]
pub enum ChatFeedIdSub {
    Channel,
    Outbox,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, PartialOrd, Ord, Hash)]
pub struct ChatFeedId(pub QualifiedChannelId, pub ChatFeedIdSub);

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, PartialOrd, Ord, Hash)]
pub enum ChatTimeId {
    Seek,
    Outbox(OutboxMessageId),
    Channel(SnapOffset),
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize, PartialOrd, Ord, Hash)]
pub struct ChatTime {
    pub stamp: Timestamp,
    pub id: ChatTimeId,
}

type ChatEntryLookup_<K> = RefCell<HashMap<K, Rc<ChatEntry>>>;

pub struct ChatEntryLookup<K: Eq + Hash>(pub Rc<ChatEntryLookup_<K>>);

impl<K: Eq + Hash> ChatEntryLookup<K> {
    pub fn new() -> Self {
        return Self(Rc::new(RefCell::new(HashMap::new())));
    }
}

pub struct ChatEntry {
    pub on_drop: ScopeValue,
    pub time: ChatTime,
    pub body: RefCell<String>,
    pub el: RefCell<Option<WeakEl>>,
}

impl ChatEntry {
    pub fn new<
        K: 'static + Eq + Hash + Clone,
    >(map: &ChatEntryLookup<K>, time: ChatTime, lookup_id: K, text: String) -> Rc<ChatEntry> {
        let out = Rc::new(ChatEntry {
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
            time: time,
            body: RefCell::new(text),
            el: RefCell::new(None),
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
        let out = el("div").text(&self.body.borrow());
        *e = Some(out.weak());
        return out;
    }

    fn time(&self) -> Self::Time {
        return self.time.clone();
    }
}
