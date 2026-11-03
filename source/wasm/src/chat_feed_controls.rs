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
    futures::channel::oneshot,
    jiff::Timestamp,
    lunk::{
        EventGraph,
        HistPrim,
    },
    rooting::spawn_rooted,
    std::{
        cell::RefCell,
        rc::Rc,
    },
};

struct FeedControls_ {
    parent: RefCell<Option<WeakInfinite<ChatEntry>>>,
    entry: Rc<ChatEntry>,
    pulling_around: RefCell<Option<oneshot::Receiver<()>>>,
    pulling_before: RefCell<Option<oneshot::Receiver<()>>>,
    pulling_after: RefCell<Option<oneshot::Receiver<()>>>,
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
            pulling_around: RefCell::new(None),
            pulling_before: RefCell::new(None),
            pulling_after: RefCell::new(None),
        }));
    }
}

impl Feed<ChatEntry> for FeedControls {
    fn set_parent(&self, parent: WeakInfinite<ChatEntry>) {
        *self.0.parent.borrow_mut() = Some(parent);
    }

    fn request_around(&self, _eg: EventGraph, time: ChatTime) {
        *self.0.pulling_before.borrow_mut() = None;
        *self.0.pulling_after.borrow_mut() = None;
        *self.0.pulling_around.borrow_mut() = Some(spawn_rooted({
            let self1 = self.clone();
            async move {
                let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                    return;
                };
                parent.respond_entries_around(&ChatFeedId::Controls, &time, vec![self1.0.entry.clone()], true, true);
            }
        }));
    }

    fn request_before(&self, _eg: EventGraph, time: ChatTime) {
        *self.0.pulling_before.borrow_mut() = Some(spawn_rooted({
            let self1 = self.clone();
            async move {
                let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                    return;
                };
                let entries = if self1.0.entry.time < time {
                    vec![self1.0.entry.clone()]
                } else {
                    vec![]
                };
                parent.respond_entries_before(&ChatFeedId::Controls, &time, entries, true);
            }
        }));
    }

    fn request_after(&self, _eg: EventGraph, time: ChatTime) {
        *self.0.pulling_after.borrow_mut() = Some(spawn_rooted({
            let self1 = self.clone();
            async move {
                let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                    return;
                };
                let entries = if self1.0.entry.time > time {
                    vec![self1.0.entry.clone()]
                } else {
                    vec![]
                };
                parent.respond_entries_after(&ChatFeedId::Controls, &time, entries, true);
            }
        }));
    }
}
