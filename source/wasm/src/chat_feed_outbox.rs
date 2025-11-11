use {
    crate::{
        chat_entry::{
            ChatEntry,
            ChatEntryLookup,
            ChatFeedId,
            ChatFeedIdSub,
            ChatTime,
            ChatTimeId,
        },
        infinite::{
            Feed,
            WeakInfinite,
        },
        outbox::OutboxMessage,
        state::{
            spawn_rooted_log,
            state,
        },
    },
    flowcontrol::{
        shed,
        ta_return,
    },
    futures::channel::oneshot,
    gloo::utils::{
        document,
        format::JsValueSerdeExt,
        window,
    },
    jiff::Timestamp,
    js_sys::Array,
    lunk::{
        EventGraph,
        ProcessingContext,
    },
    rooting::ScopeValue,
    serde::de::DeserializeOwned,
    shared::interface::shared::{
        ChannelId,
        OutboxMessageId,
        QualifiedChannelId,
    },
    std::{
        cell::RefCell,
        collections::BTreeMap,
        ops::Bound,
        rc::Rc,
        str::FromStr,
    },
    tokio_stream::StreamExt,
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
    wasm_bindgen_futures::{
        stream::JsStream,
        JsFuture,
    },
    web_sys::{
        FileSystemDirectoryHandle,
        FileSystemFileHandle,
        FileSystemGetDirectoryOptions,
    },
};

struct OutboxFeed_ {
    id: QualifiedChannelId,
    entry_lookup: ChatEntryLookup<OutboxMessageId>,
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
            let Some(pivot1) = parent.want_after(ChatFeedId(self.0.id.clone(), ChatFeedIdSub::Outbox)) else {
                return;
            };
            pivot = pivot1;
        }
        self.request_after(eg, pivot);
    }
}

async fn opfs_ensure_dir(parent: &FileSystemDirectoryHandle, seg: &str) -> FileSystemDirectoryHandle {
    return JsFuture::from(parent.get_directory_handle_with_options(seg, &{
        let x = FileSystemGetDirectoryOptions::new();
        x.set_create(true);
        x
    }))
        .await
        .expect("Error getting/creating opfs dir")
        .dyn_into::<FileSystemDirectoryHandle>()
        .expect("Opfs get dir handle result wasn't file system dir handle");
}

async fn opfs_list_dir(parent: &FileSystemDirectoryHandle) -> Vec<(String, JsValue)> {
    let mut entries = vec![];
    let mut entries0 = JsStream::from(parent.entries());
    while let Some(e) = entries0.next().await {
        let e = match e {
            Ok(e) => e,
            Err(e) => {
                state().log.log_js2("Error reading directory entry", parent, &e);
                continue;
            },
        };
        let e = e.dyn_into::<Array>().unwrap();
        let name = e.get(0).as_string().unwrap();
        let handle = e.get(1);
        entries.push((name, handle));
    }
    return entries;
}

async fn opfs_read_json<T: DeserializeOwned>(parent: &FileSystemDirectoryHandle, seg: &str) -> Result<T, String> {
    return Ok(
        serde_json::from_str::<T>(
            &JsFuture::from(
                web_sys::File::from(
                    JsFuture::from(
                        FileSystemFileHandle::from(
                            JsFuture::from(parent.get_file_handle(seg))
                                .await
                                .map_err(
                                    |e| format!("Error getting file handle at seg [{}]: {:?}", seg, e.as_string()),
                                )?,
                        ).get_file(),
                    )
                        .await
                        .map_err(
                            |e| format!("Error getting file from file handle at seg [{}]: {:?}", seg, e.as_string()),
                        )?,
                ).text(),
            )
                .await
                .map_err(|e| format!("Error getting string contents of file at seg [{}]: {:?}", seg, e.as_string()))?
                .as_string()
                .unwrap(),
        ).map_err(|e| format!("Error parsing json file from opfs at seg [{}]: {}", seg, e))?,
    );
}

async fn opfs_channel_dir(channel: &QualifiedChannelId) -> BTreeMap<Timestamp, FileSystemDirectoryHandle> {
    let opfs =
        JsFuture::from(window().navigator().storage().get_directory())
            .await
            .expect("Error getting opfs root")
            .dyn_into::<FileSystemDirectoryHandle>()
            .unwrap();
    let opfs = opfs_ensure_dir(&opfs, "outbox").await;
    let opfs = opfs_ensure_dir(&opfs, &channel.identity.to_string()).await;
    let opfs = opfs_ensure_dir(&opfs, &channel.channel.0.to_string()).await;
    let entries0 = opfs_list_dir(&opfs).await;
    let mut entries = BTreeMap::new();
    for (name, handle) in entries0 {
        let name = match Timestamp::from_str(&name) {
            Ok(n) => n,
            Err(e) => {
                state().log.log(&format!("Outbox entry [{}] is not a valid timestamp, skipping: {}", name, e));
                continue;
            },
        };
        let Some(handle) = handle.dyn_ref::<FileSystemDirectoryHandle>() else {
            state().log.log_js("Outbox entry is not a directory, skipping", &handle);
            continue;
        };
        entries.insert(name, handle.clone());
    }
    return entries;
}

const WANT_COUNT: usize = 10;
const OPFS_FILENAME_MAIN: &str = "main";

impl Feed<ChatEntry> for OutboxFeed {
    fn set_parent(&self, parent: WeakInfinite<ChatEntry>) {
        *self.0.parent.borrow_mut() = Some(parent);
    }

    fn request_around(&self, eg: EventGraph, time: ChatTime) {
        *self.0.pulling_after.borrow_mut() = None;
        *self.0.pulling_before.borrow_mut() = None;
        *self.0.pulling_around.borrow_mut() = Some(spawn_rooted_log("Outbox feed, request around", {
            let self1 = self.clone();
            async move {
                let opfs_entries = opfs_channel_dir(&self1.0.id).await;
                let mut early_entries = vec![];
                for (
                    k,
                    v,
                ) in opfs_entries.range((Bound::Unbounded, Bound::Included(time.stamp))).rev().take(WANT_COUNT) {
                    match async {
                        ta_return!((), String);
                        let main = opfs_read_json::<OutboxMessage>(v, OPFS_FILENAME_MAIN).await?;
                        early_entries.push(ChatEntry::new(&self1.0.entry_lookup, ChatTime {
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
                        late_entries.push(ChatEntry::new(&self1.0.entry_lookup, ChatTime {
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
                eg.event(|pc| {
                    let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                        return;
                    };
                    let early_stop = early_entries.len() < WANT_COUNT;
                    let late_stop = late_entries.len() < WANT_COUNT;
                    early_entries.reverse();
                    early_entries.extend(late_entries);
                    parent.respond_entries_around(
                        &ChatFeedId(self1.0.id.clone(), ChatFeedIdSub::Outbox),
                        &time,
                        early_entries,
                        early_stop,
                        late_stop,
                    );
                });
                return Ok(());
            }
        }));
    }

    fn request_before(&self, eg: EventGraph, time: ChatTime) {
        *self.0.pulling_before.borrow_mut() = Some(spawn_rooted_log("Outbox feed, request before", {
            let self1 = self.clone();
            async move {
                let opfs_entries = opfs_channel_dir(&self1.0.id).await;
                let mut entries = vec![];
                for (k, v) in opfs_entries.range((Bound::Unbounded, Bound::Excluded(time.stamp))).rev().take(10) {
                    match async {
                        ta_return!((), String);
                        let main = opfs_read_json::<OutboxMessage>(v, OPFS_FILENAME_MAIN).await?;
                        entries.push(ChatEntry::new(&self1.0.entry_lookup, ChatTime {
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
                eg.event(|pc| {
                    let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                        return;
                    };
                    let stop = entries.len() < WANT_COUNT;
                    parent.respond_entries_before(
                        &ChatFeedId(self1.0.id.clone(), ChatFeedIdSub::Outbox),
                        &time,
                        entries,
                        stop,
                    );
                });
                return Ok(());
            }
        }));
    }

    fn request_after(&self, eg: EventGraph, time: ChatTime) {
        *self.0.pulling_after.borrow_mut() = Some(spawn_rooted_log("Outbox feed, request after", {
            let self1 = self.clone();
            async move {
                let opfs_entries = opfs_channel_dir(&self1.0.id).await;
                const WANT_COUNT: usize = 10;
                let mut entries = vec![];
                for (k, v) in opfs_entries.range((Bound::Unbounded, Bound::Excluded(time.stamp))).take(10) {
                    match async {
                        ta_return!((), String);
                        let main = opfs_read_json::<OutboxMessage>(v, OPFS_FILENAME_MAIN).await?;
                        entries.push(ChatEntry::new(&self1.0.entry_lookup, ChatTime {
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
                eg.event(|pc| {
                    let Some(parent) = self1.0.parent.borrow().as_ref().and_then(|p| p.upgrade()) else {
                        return;
                    };
                    let stop = entries.len() < WANT_COUNT;
                    parent.respond_entries_after(
                        &ChatFeedId(self1.0.id.clone(), ChatFeedIdSub::Outbox),
                        &time,
                        entries,
                        stop,
                    );
                });
                return Ok(());
            }
        }));
    }
}
