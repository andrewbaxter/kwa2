use {
    crate::{
        chat::{
            ChatMode,
            ChatState,
        },
        chat_entry::{
            ChatEntry,
            ChatEntryControls,
            ChatEntryInternal,
            ChatFeedId,
            ChatTime,
            ChatTimeId,
        },
        infinite::{
            Feed,
            WeakInfinite,
        },
    },
    jiff::Timestamp,
    lunk::{
        EventGraph,
        HistPrim,
    },
    std::{
        cell::RefCell,
        rc::Rc,
    },
};

struct FeedControls_ {
    parent: RefCell<Option<WeakInfinite<ChatEntry>>>,
    entry: Rc<ChatEntry>,
}

#[derive(Clone)]
pub struct FeedControls(Rc<FeedControls_>);

impl FeedControls {
    pub fn new(mode: HistPrim<ChatMode>, state: Rc<ChatState>) -> FeedControls {
        return FeedControls(Rc::new(FeedControls_ {
            parent: Default::default(),
            entry: Rc::new(ChatEntry {
                time: ChatTime {
                    stamp: Timestamp::MAX,
                    id: ChatTimeId::None,
                },
                int: ChatEntryInternal::Controls(ChatEntryControls {
                    mode: mode,
                    state: state,
                }),
                el: Default::default(),
            }),
        }));
    }
}

impl Feed<ChatEntry> for FeedControls {
    fn set_parent(&self, parent: WeakInfinite<ChatEntry>) {
        *self.0.parent.borrow_mut() = Some(parent);
    }

    fn request_around(&self, _eg: EventGraph, time: ChatTime) {
        let Some(parent) = self.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
            return;
        };
        if time <= self.0.entry.time {
            return;
        }
        parent.respond_entries_around(&ChatFeedId::Controls, &time, vec![self.0.entry.clone()], true, true);
    }

    fn request_before(&self, _eg: EventGraph, time: ChatTime) {
        let Some(parent) = self.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
            return;
        };
        if time <= self.0.entry.time {
            return;
        }
        parent.respond_entries_before(&ChatFeedId::Controls, &time, vec![self.0.entry.clone()], true);
    }

    fn request_after(&self, _eg: EventGraph, time: ChatTime) {
        let Some(parent) = self.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
            return;
        };
        if time >= self.0.entry.time {
            return;
        }
        parent.respond_entries_after(&ChatFeedId::Controls, &time, vec![self.0.entry.clone()], true);
    }
}
