use {
    crate::{
        chat_entry::{
            ChatEntry,
            ChatEntryControls,
            ChatMode,
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
    shared::interface::shared::QualifiedChannelId,
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
    pub fn new(
        controls_group: HistPrim<ChatMode>,
        channels: Vec<(String, QualifiedChannelId)>,
    ) -> FeedControls {
        let channels = Rc::new(RefCell::new(channels));
        return FeedControls(Rc::new(FeedControls_ {
            parent: Default::default(),
            channels: channels.clone(),
            entry: Rc::new(ChatEntry {
                time: ChatTime {
                    stamp: Timestamp::MAX,
                    id: ChatTimeId::None,
                },
                int: ChatEntryInternal::Controls(ChatEntryControls {
                    group_mode: controls_group,
                    channels: channels,
                }),
                el: Default::default(),
            }),
        }));
    }

    pub fn add_channel(&self, memo: String, id: QualifiedChannelId) {
        self.0.channels.borrow_mut().push((memo, id));
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
