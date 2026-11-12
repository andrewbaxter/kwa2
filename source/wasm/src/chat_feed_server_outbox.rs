use {
    crate::{
        blocks::view_blocks,
        api::req_get_retry,
        chat::{
            ChatState,
            CopyStage,
            new_message_copy,
        },
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
        outbox::opfs_delete_outbox_message,
        state::spawn_rooted_log,
    },
    futures::channel::oneshot,
    lunk::EventGraph,
    shared::interface::{
        shared::{
            MessageRel,
            QualifiedChannelId,
            QualifiedMessageId,
        },
        wire::c2s::{
            self,
            OutboxEntry,
        },
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        collections::BTreeMap,
        ops::Bound,
        rc::Rc,
    },
};

struct ServerOutboxFeed_ {
    id: QualifiedChannelId,
    sender: Identity,
    chat_state: Rc<ChatState>,
    parent: RefCell<Option<WeakInfinite<ChatEntry>>>,
    pulling_around: RefCell<Option<oneshot::Receiver<()>>>,
    pulling_before: RefCell<Option<oneshot::Receiver<()>>>,
    pulling_after: RefCell<Option<oneshot::Receiver<()>>>,
}

#[derive(Clone)]
pub struct ServerOutboxFeed(Rc<ServerOutboxFeed_>);

impl ServerOutboxFeed {
    pub fn new(chat_state: Rc<ChatState>, sender: Identity, id: QualifiedChannelId) -> ServerOutboxFeed {
        return ServerOutboxFeed(Rc::new(ServerOutboxFeed_ {
            id: id,
            sender: sender,
            chat_state: chat_state,
            parent: RefCell::new(None),
            pulling_around: RefCell::new(None),
            pulling_before: RefCell::new(None),
            pulling_after: RefCell::new(None),
        }));
    }

    fn feed_id(&self) -> ChatFeedId {
        return ChatFeedId::ServerOutbox((self.0.sender.clone(), self.0.id.clone()));
    }

    pub fn notify(&self, eg: EventGraph) {
        let Some(parent) = self.0.parent.borrow().as_ref().unwrap().upgrade() else {
            return;
        };
        let Some(pivot) = parent.want_after(self.feed_id()) else {
            return;
        };
        self.request_after(eg, pivot);
    }

    async fn load_sources(&self) -> Result<BTreeMap<ChatTime, OutboxEntry>, String> {
        let mut out = BTreeMap::new();
        let mut edits = vec![];
        for e in req_get_retry(c2s::OutboxList { channel: self.0.id.clone() }).await? {
            if e.sender != self.0.sender {
                continue;
            }
            if let MessageRel::EditOf(target) = &e.rel {
                edits.push((QualifiedMessageId {
                    channel: self.0.id.clone(),
                    message: target.clone(),
                }, e.client_id, view_blocks(&self.0.id, e.blocks)));
                continue;
            }
            out.insert(ChatTime {
                stamp: e.created,
                id: ChatTimeId::ServerOutbox(e.id.unique),
            }, e);
        }
        if !edits.is_empty() {
            self.0.chat_state.eg.event(|pc| {
                for (target, client_id, blocks) in &edits {
                    self.0.chat_state.apply_edit_overlay(pc, target.clone(), client_id, blocks.clone());
                }
            }).unwrap();
            for (_, client_id, _) in &edits {
                opfs_delete_outbox_message(&self.0.sender, &self.0.id, client_id).await;
            }
        }
        return Ok(out);
    }

    async fn create_entries<
        'a,
    >(&self, sources: impl Iterator<Item = (&'a ChatTime, &'a OutboxEntry)>) -> Vec<Rc<ChatEntry>> {
        let mut out = vec![];
        let mut client_ids = vec![];
        for (time, e) in sources {
            out.push(
                new_message_copy(
                    &self.0.chat_state,
                    CopyStage::Server,
                    (self.0.id.clone(), self.0.sender.clone(), e.client_id.clone()),
                    None,
                    time.clone(),
                    view_blocks(&self.0.id, e.blocks.clone()),
                ),
            );
            client_ids.push(e.client_id.clone());
        }
        for client_id in &client_ids {
            opfs_delete_outbox_message(&self.0.sender, &self.0.id, client_id).await;
        }
        return out;
    }
}

const WANT_COUNT: usize = 10;

impl Feed<ChatEntry> for ServerOutboxFeed {
    fn set_parent(&self, parent: WeakInfinite<ChatEntry>) {
        *self.0.parent.borrow_mut() = Some(parent);
    }

    fn request_around(&self, _eg: EventGraph, time: ChatTime) {
        *self.0.pulling_after.borrow_mut() = None;
        *self.0.pulling_before.borrow_mut() = None;
        *self.0.pulling_around.borrow_mut() = Some(spawn_rooted_log("Server outbox feed, request around", {
            let self1 = self.clone();
            async move {
                let sources = self1.load_sources().await?;
                let mut early_entries =
                    self1
                        .create_entries(
                            sources.range((Bound::Unbounded, Bound::Included(&time))).rev().take(WANT_COUNT),
                        )
                        .await;
                let late_entries =
                    self1
                        .create_entries(sources.range((Bound::Excluded(&time), Bound::Unbounded)).take(WANT_COUNT))
                        .await;
                let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                    return Ok(());
                };
                let early_stop = early_entries.len() < WANT_COUNT;
                let late_stop = late_entries.len() < WANT_COUNT;
                early_entries.reverse();
                early_entries.extend(late_entries);
                parent.respond_entries_around(&self1.feed_id(), &time, early_entries, early_stop, late_stop);
                return Ok(());
            }
        }));
    }

    fn request_before(&self, _eg: EventGraph, time: ChatTime) {
        *self.0.pulling_before.borrow_mut() = Some(spawn_rooted_log("Server outbox feed, request before", {
            let self1 = self.clone();
            async move {
                let sources = self1.load_sources().await?;
                let entries =
                    self1
                        .create_entries(
                            sources.range((Bound::Unbounded, Bound::Excluded(&time))).rev().take(WANT_COUNT),
                        )
                        .await;
                let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                    return Ok(());
                };
                let stop = entries.len() < WANT_COUNT;
                parent.respond_entries_before(&self1.feed_id(), &time, entries, stop);
                return Ok(());
            }
        }));
    }

    fn request_after(&self, _eg: EventGraph, time: ChatTime) {
        *self.0.pulling_after.borrow_mut() = Some(spawn_rooted_log("Server outbox feed, request after", {
            let self1 = self.clone();
            async move {
                let sources = self1.load_sources().await?;
                let entries =
                    self1
                        .create_entries(sources.range((Bound::Excluded(&time), Bound::Unbounded)).take(WANT_COUNT))
                        .await;
                let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                    return Ok(());
                };
                let stop = entries.len() < WANT_COUNT;
                parent.respond_entries_after(&self1.feed_id(), &time, entries, stop);
                return Ok(());
            }
        }));
    }
}
