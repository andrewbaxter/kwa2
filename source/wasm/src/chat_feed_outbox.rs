use {
    crate::{
        chat::ChatState,
        chat_entry::{
            ChatEntry,
            ChatFeedId,
            ChatTime,
            ChatTimeId,
        },
        infinite::{
            Feed,
            WeakInfinite,
        },
        outbox::{
            OPFS_FILENAME_MAIN,
            OutboxMessage,
            opfs_channel_dir_entries,
            opfs_outbox_channel_dir,
            opfs_read_json,
        },
        state::{
            spawn_rooted_log,
            state,
        },
    },
    defer::defer,
    flowcontrol::ta_return,
    futures::channel::oneshot,
    jiff::Timestamp,
    lunk::EventGraph,
    rooting::scope_any,
    shared::interface::shared::QualifiedChannelId,
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        ops::Bound,
        rc::Rc,
    },
};

struct OutboxFeed_ {
    id: QualifiedChannelId,
    sender: Identity,
    chat_state: Rc<ChatState>,
    parent: RefCell<Option<WeakInfinite<ChatEntry>>>,
    pulling_around: RefCell<Option<oneshot::Receiver<()>>>,
    pulling_before: RefCell<Option<oneshot::Receiver<()>>>,
    pulling_after: RefCell<Option<oneshot::Receiver<()>>>,
}

#[derive(Clone)]
pub struct OutboxFeed(Rc<OutboxFeed_>);

impl OutboxFeed {
    pub fn new(chat_state: Rc<ChatState>, sender: Identity, id: QualifiedChannelId) -> OutboxFeed {
        return OutboxFeed(Rc::new(OutboxFeed_ {
            id: id,
            sender: sender,
            chat_state: chat_state,
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
            let Some(pivot1) =
                parent.want_after(ChatFeedId::Outbox((self.0.sender.clone(), self.0.id.clone()))) else {
                    return;
                };
            pivot = pivot1;
        }
        self.request_after(eg, pivot);
    }
}

const WANT_COUNT: usize = 10;

fn create_entry(
    chat_state: &Rc<ChatState>,
    channel: &QualifiedChannelId,
    sender: &Identity,
    k: &Timestamp,
    main: OutboxMessage,
) -> Rc<ChatEntry> {
    let entry = ChatEntry::new_message(ChatTime {
        stamp: *k,
        id: ChatTimeId::Outbox(main.client_id.clone()),
    }, main.body, scope_any(defer({
        let channel = channel.clone();
        let idem = main.client_id.clone();
        let sender = sender.clone();
        let chat_state = Rc::downgrade(chat_state);
        move || {
            let Some(chat_state) = chat_state.upgrade() else {
                return;
            };
            chat_state.entry_outbox_lookup.borrow_mut().remove(&(channel.clone(), sender.clone(), idem.clone()));
        }
    })));
    chat_state
        .entry_outbox_lookup
        .borrow_mut()
        .insert((channel.clone(), sender.clone(), main.client_id.clone()), entry.clone());
    return entry;
}

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
                let opfs_root = opfs_outbox_channel_dir(&self1.0.sender, &self1.0.id).await;
                let opfs_entries = opfs_channel_dir_entries(&opfs_root).await;
                let mut early_entries = vec![];
                for (
                    k,
                    v,
                ) in opfs_entries.range((Bound::Unbounded, Bound::Included(time.stamp))).rev().take(WANT_COUNT) {
                    match async {
                        ta_return!((), String);
                        let main = opfs_read_json::<OutboxMessage>(v, OPFS_FILENAME_MAIN).await?;
                        early_entries.push(create_entry(&self1.0.chat_state, &self1.0.id, &self1.0.sender, k, main));
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
                        late_entries.push(create_entry(&self1.0.chat_state, &self1.0.id, &self1.0.sender, k, main));
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
                    &ChatFeedId::Outbox((self1.0.sender.clone(), self1.0.id.clone())),
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
                let opfs_channel_dir = opfs_outbox_channel_dir(&self1.0.sender, &self1.0.id).await;
                let opfs_entries = opfs_channel_dir_entries(&opfs_channel_dir).await;
                let mut entries = vec![];
                for (k, v) in opfs_entries.range((Bound::Unbounded, Bound::Excluded(time.stamp))).rev().take(10) {
                    match async {
                        ta_return!((), String);
                        let main = opfs_read_json::<OutboxMessage>(v, OPFS_FILENAME_MAIN).await?;
                        entries.push(create_entry(&self1.0.chat_state, &self1.0.id, &self1.0.sender, k, main));
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
                parent.respond_entries_before(
                    &ChatFeedId::Outbox((self1.0.sender.clone(), self1.0.id.clone())),
                    &time,
                    entries,
                    stop,
                );
                return Ok(());
            }
        }));
    }

    fn request_after(&self, _eg: EventGraph, time: ChatTime) {
        *self.0.pulling_after.borrow_mut() = Some(spawn_rooted_log("Outbox feed, request after", {
            let self1 = self.clone();
            async move {
                let opfs_channel_dir = opfs_outbox_channel_dir(&self1.0.sender, &self1.0.id).await;
                let opfs_entries = opfs_channel_dir_entries(&opfs_channel_dir).await;
                const WANT_COUNT: usize = 10;
                let mut entries = vec![];
                for (k, v) in opfs_entries.range((Bound::Unbounded, Bound::Excluded(time.stamp))).take(10) {
                    match async {
                        ta_return!((), String);
                        let main = opfs_read_json::<OutboxMessage>(v, OPFS_FILENAME_MAIN).await?;
                        entries.push(create_entry(&self1.0.chat_state, &self1.0.id, &self1.0.sender, k, main));
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
                parent.respond_entries_after(
                    &ChatFeedId::Outbox((self1.0.sender.clone(), self1.0.id.clone())),
                    &time,
                    entries,
                    stop,
                );
                return Ok(());
            }
        }));
    }
}
