use {
    crate::{
        chat_entry::{
            ChatEntry,
            ChatEntryLookup,
            ChatFeedId,
            ChatTime,
            ChatTimeId,
        },
        infinite::{
            Feed,
            WeakInfinite,
        },
        outbox::{
            opfs_channel_dir,
            opfs_channel_dir_entries,
            opfs_read_json,
            OutboxMessage,
            OPFS_FILENAME_MAIN,
        },
        state::{
            spawn_rooted_log,
            state,
        },
    },
    flowcontrol::ta_return,
    futures::channel::oneshot,
    lunk::EventGraph,
    shared::interface::shared::{
        MessageIdem,
        QualifiedChannelId,
    },
    std::{
        cell::RefCell,
        ops::Bound,
        rc::Rc,
    },
};

struct OutboxFeed_ {
    id: QualifiedChannelId,
    entry_lookup: ChatEntryLookup<MessageIdem>,
    parent: RefCell<Option<WeakInfinite<ChatEntry>>>,
    pulling_around: RefCell<Option<oneshot::Receiver<()>>>,
    pulling_before: RefCell<Option<oneshot::Receiver<()>>>,
    pulling_after: RefCell<Option<oneshot::Receiver<()>>>,
}

#[derive(Clone)]
pub struct OutboxFeed(Rc<OutboxFeed_>);

impl OutboxFeed {
    pub fn new(id: QualifiedChannelId) -> OutboxFeed {
        return OutboxFeed(Rc::new(OutboxFeed_ {
            id: id,
            entry_lookup: ChatEntryLookup::new(),
            parent: RefCell::new(None),
            pulling_around: RefCell::new(None),
            pulling_before: RefCell::new(None),
            pulling_after: RefCell::new(None),
        }));
    }

    pub fn notify(&self, eg: EventGraph) {
        let pivot;
        {
            let Some(parent) = self.0.parent.borrow().as_ref().unwrap().upgrade() else {
                return;
            };
            let Some(pivot1) = parent.want_after(ChatFeedId::Outbox(self.0.id.clone())) else {
                return;
            };
            pivot = pivot1;
        }
        self.request_after(eg, pivot);
    }
}

const WANT_COUNT: usize = 10;

impl Feed<ChatEntry> for OutboxFeed {
    fn set_parent(&self, parent: WeakInfinite<ChatEntry>) {
        *self.0.parent.borrow_mut() = Some(parent);
    }

    fn request_around(&self, _eg: EventGraph, time: ChatTime) {
        *self.0.pulling_after.borrow_mut() = None;
        *self.0.pulling_before.borrow_mut() = None;
        *self.0.pulling_around.borrow_mut() = Some(spawn_rooted_log("Outbox feed, request around", {
            let self1 = self.clone();
            async move {
                let opfs_root = opfs_channel_dir(&self1.0.id).await;
                let opfs_entries = opfs_channel_dir_entries(&opfs_root).await;
                let mut early_entries = vec![];
                for (
                    k,
                    v,
                ) in opfs_entries.range((Bound::Unbounded, Bound::Included(time.stamp))).rev().take(WANT_COUNT) {
                    match async {
                        ta_return!((), String);
                        let main = opfs_read_json::<OutboxMessage>(v, OPFS_FILENAME_MAIN).await?;
                        early_entries.push(ChatEntry::new_message(&self1.0.entry_lookup, ChatTime {
                            stamp: *k,
                            id: ChatTimeId::Outbox(main.idem.clone()),
                        }, main.idem, main.body));
                        return Ok(());
                    }.await {
                        Ok(()) => { },
                        Err(e) => {
                            state().log.log(&format!("Error processing outbox entry, skipping: {}", e));
                            continue;
                        },
                    };
                }
                let mut late_entries = vec![];
                for (
                    k,
                    v,
                ) in opfs_entries.range((Bound::Excluded(time.stamp), Bound::Unbounded)).take(WANT_COUNT) {
                    match async {
                        ta_return!((), String);
                        let main = opfs_read_json::<OutboxMessage>(v, OPFS_FILENAME_MAIN).await?;
                        late_entries.push(ChatEntry::new_message(&self1.0.entry_lookup, ChatTime {
                            stamp: *k,
                            id: ChatTimeId::Outbox(main.idem.clone()),
                        }, main.idem, main.body));
                        return Ok(());
                    }.await {
                        Ok(()) => { },
                        Err(e) => {
                            state().log.log(&format!("Error processing outbox entry, skipping: {}", e));
                            continue;
                        },
                    };
                }
                let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                    return Ok(());
                };
                let early_stop = early_entries.len() < WANT_COUNT;
                let late_stop = late_entries.len() < WANT_COUNT;
                early_entries.reverse();
                early_entries.extend(late_entries);
                parent.respond_entries_around(
                    &ChatFeedId::Outbox(self1.0.id.clone()),
                    &time,
                    early_entries,
                    early_stop,
                    late_stop,
                );
                return Ok(());
            }
        }));
    }

    fn request_before(&self, _eg: EventGraph, time: ChatTime) {
        *self.0.pulling_before.borrow_mut() = Some(spawn_rooted_log("Outbox feed, request before", {
            let self1 = self.clone();
            async move {
                let opfs_channel_dir = opfs_channel_dir(&self1.0.id).await;
                let opfs_entries = opfs_channel_dir_entries(&opfs_channel_dir).await;
                let mut entries = vec![];
                for (k, v) in opfs_entries.range((Bound::Unbounded, Bound::Excluded(time.stamp))).rev().take(10) {
                    match async {
                        ta_return!((), String);
                        let main = opfs_read_json::<OutboxMessage>(v, OPFS_FILENAME_MAIN).await?;
                        entries.push(ChatEntry::new_message(&self1.0.entry_lookup, ChatTime {
                            stamp: *k,
                            id: ChatTimeId::Outbox(main.idem.clone()),
                        }, main.idem, main.body));
                        return Ok(());
                    }.await {
                        Ok(()) => { },
                        Err(e) => {
                            state().log.log(&format!("Error processing outbox entry, skipping: {}", e));
                            continue;
                        },
                    };
                }
                let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                    return Ok(());
                };
                let stop = entries.len() < WANT_COUNT;
                parent.respond_entries_before(&ChatFeedId::Outbox(self1.0.id.clone()), &time, entries, stop);
                return Ok(());
            }
        }));
    }

    fn request_after(&self, _eg: EventGraph, time: ChatTime) {
        *self.0.pulling_after.borrow_mut() = Some(spawn_rooted_log("Outbox feed, request after", {
            let self1 = self.clone();
            async move {
                let opfs_channel_dir = opfs_channel_dir(&self1.0.id).await;
                let opfs_entries = opfs_channel_dir_entries(&opfs_channel_dir).await;
                const WANT_COUNT: usize = 10;
                let mut entries = vec![];
                for (k, v) in opfs_entries.range((Bound::Unbounded, Bound::Excluded(time.stamp))).take(10) {
                    match async {
                        ta_return!((), String);
                        let main = opfs_read_json::<OutboxMessage>(v, OPFS_FILENAME_MAIN).await?;
                        entries.push(ChatEntry::new_message(&self1.0.entry_lookup, ChatTime {
                            stamp: *k,
                            id: ChatTimeId::Outbox(main.idem.clone()),
                        }, main.idem, main.body));
                        return Ok(());
                    }.await {
                        Ok(()) => { },
                        Err(e) => {
                            state().log.log(&format!("Error processing outbox entry, skipping: {}", e));
                            continue;
                        },
                    };
                }
                let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                    return Ok(());
                };
                let stop = entries.len() < WANT_COUNT;
                parent.respond_entries_after(&ChatFeedId::Outbox(self1.0.id.clone()), &time, entries, stop);
                return Ok(());
            }
        }));
    }
}
