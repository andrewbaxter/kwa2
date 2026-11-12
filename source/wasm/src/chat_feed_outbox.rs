use {
    crate::{
        blocks::{
            LocalBlob,
            ViewBlock,
            ViewFile,
            ViewFileSource,
        },
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
        opfs::{
            opfs_read_file,
            opfs_read_json,
        },
        outbox::{
            OPFS_FILENAME_MAIN,
            OutboxBlock,
            OutboxFile,
            OutboxMessage,
            OutboxMessageRel,
            opfs_channel_dir_entries,
            opfs_outbox_channel_dir,
        },
        state::{
            spawn_rooted_log,
            state,
        },
    },
    flowcontrol::ta_return,
    futures::channel::oneshot,
    jiff::Timestamp,
    lunk::EventGraph,
    shared::interface::shared::{
        MessageClientId,
        QualifiedChannelId,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        collections::BTreeMap,
        ops::Bound,
        rc::Rc,
        str::FromStr,
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

    fn feed_id(&self) -> ChatFeedId {
        return ChatFeedId::Outbox((self.0.sender.clone(), self.0.id.clone()));
    }

    pub fn add_sent(&self, client_id: MessageClientId, blocks: Vec<ViewBlock>) {
        let parent = self.0.parent.borrow().as_ref().unwrap().upgrade().unwrap();
        let Some(pivot) = parent.want_after(self.feed_id()) else {
            return;
        };
        let stamp = Timestamp::from_str(&client_id.0).unwrap();
        parent.respond_entries_after(
            &self.feed_id(),
            &pivot,
            vec![self.create_entry(&stamp, client_id, blocks)],
            true,
        );
    }

    fn create_entry(&self, k: &Timestamp, client_id: MessageClientId, blocks: Vec<ViewBlock>) -> Rc<ChatEntry> {
        return new_message_copy(
            &self.0.chat_state,
            CopyStage::Device,
            (self.0.id.clone(), self.0.sender.clone(), client_id.clone()),
            None,
            ChatTime {
                stamp: *k,
                id: ChatTimeId::Outbox(client_id),
            },
            blocks,
        );
    }

    async fn load_sources(&self) -> BTreeMap<Timestamp, (MessageClientId, Vec<ViewBlock>)> {
        let entries = opfs_channel_dir_entries(&opfs_outbox_channel_dir(&self.0.sender, &self.0.id).await).await;
        let mut out = BTreeMap::new();
        let mut edits = vec![];
        for (k, v) in entries {
            let res = async {
                ta_return!((OutboxMessage, Vec < ViewBlock >), String);
                let main = opfs_read_json::<OutboxMessage>(&v, OPFS_FILENAME_MAIN).await?;
                let mut blocks = vec![];
                for block in &main.blocks {
                    match block {
                        OutboxBlock::Text(t) => blocks.push(ViewBlock::Text(t.clone())),
                        OutboxBlock::Extension(e) => blocks.push(ViewBlock::Extension(e.clone())),
                        OutboxBlock::File(f) => blocks.push(ViewBlock::File(ViewFile {
                            source: match &f.file {
                                OutboxFile::Local(seg) => ViewFileSource::Local(
                                    LocalBlob::new(opfs_read_file(&v, seg).await?.into()),
                                ),
                                OutboxFile::Remote(file) => ViewFileSource::Remote {
                                    channel: self.0.id.clone(),
                                    file: file.clone(),
                                },
                            },
                            mime_type: f.mime_type.clone(),
                            size: f.size,
                            name: f.name.clone(),
                        })),
                    }
                }
                return Ok((main, blocks));
            }.await;
            match res {
                Ok((main, blocks)) => match &main.rel {
                    OutboxMessageRel::EditOf(target) => {
                        edits.push((target.clone(), main.client_id, blocks));
                    },
                    OutboxMessageRel::None | OutboxMessageRel::ReplyTo(_) => {
                        out.insert(k, (main.client_id, blocks));
                    },
                },
                Err(e) => {
                    state().log.log(&format!("Error processing outbox entry, skipping: {}", e));
                },
            }
        }
        if !edits.is_empty() {
            self.0.chat_state.eg.event(|pc| {
                for (target, client_id, blocks) in edits {
                    self.0.chat_state.apply_edit_overlay(pc, target, &client_id, blocks);
                }
            }).unwrap();
        }
        return out;
    }

    fn create_entries<
        'a,
    >(
        &self,
        sources: impl Iterator<Item = (&'a Timestamp, &'a (MessageClientId, Vec<ViewBlock>))>,
        limit: usize,
    ) -> Vec<Rc<ChatEntry>> {
        return sources
            .take(limit)
            .map(|(k, (client_id, blocks))| self.create_entry(k, client_id.clone(), blocks.clone()))
            .collect();
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
                let sources = self1.load_sources().await;
                let mut early_entries =
                    self1.create_entries(
                        sources.range((Bound::Unbounded, Bound::Included(time.stamp))).rev(),
                        WANT_COUNT,
                    );
                let late_entries =
                    self1.create_entries(
                        sources.range((Bound::Excluded(time.stamp), Bound::Unbounded)),
                        WANT_COUNT,
                    );
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
        *self.0.pulling_before.borrow_mut() = Some(spawn_rooted_log("Outbox feed, request before", {
            let self1 = self.clone();
            async move {
                let sources = self1.load_sources().await;
                let entries =
                    self1.create_entries(
                        sources.range((Bound::Unbounded, Bound::Excluded(time.stamp))).rev(),
                        WANT_COUNT,
                    );
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
        *self.0.pulling_after.borrow_mut() = Some(spawn_rooted_log("Outbox feed, request after", {
            let self1 = self.clone();
            async move {
                let sources = self1.load_sources().await;
                let entries =
                    self1.create_entries(
                        sources.range((Bound::Excluded(time.stamp), Bound::Unbounded)),
                        WANT_COUNT,
                    );
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
