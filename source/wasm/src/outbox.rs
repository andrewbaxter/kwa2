use {
    crate::{
        background::trigger_push,
        blocks::{
            ViewBlock,
            ViewFileSource,
        },
        opfs::{
            opfs_delete,
            opfs_ensure_dir,
            opfs_list_dir,
            opfs_root,
            opfs_write_json,
        },
        state::state,
    },
    gloo::storage::{
        LocalStorage,
        Storage,
        errors::StorageError,
    },
    jiff::{
        SignedDuration,
        Timestamp,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::shared::{
        MessageBlockEvent,
        MessageBlockExtension,
        MessageClientId,
        QualifiedChannelId,
        QualifiedFileId,
        QualifiedMessageId,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        collections::BTreeMap,
        str::FromStr,
    },
    wasm_bindgen::JsCast,
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        FileSystemDirectoryHandle,
        FileSystemFileHandle,
        FileSystemGetFileOptions,
        FileSystemWritableFileStream,
    },
};

pub const OPFS_FILENAME_MAIN: &str = "main";
const LOCALSTORAGE_LAST_CLIENT_ID_STAMP: &str = "last_client_id_stamp";

pub fn new_message_client_id() -> Result<MessageClientId, String> {
    let last = match LocalStorage::get::<Timestamp>(LOCALSTORAGE_LAST_CLIENT_ID_STAMP) {
        Ok(t) => Some(t),
        Err(StorageError::KeyNotFound(_)) => None,
        Err(e) => return Err(format!("Error reading last message client id timestamp: {}", e)),
    };
    let now = Timestamp::now();
    let stamp = match last {
        Some(last) if now <= last => last + SignedDuration::from_nanos(1),
        _ => now,
    };
    LocalStorage::set(
        LOCALSTORAGE_LAST_CLIENT_ID_STAMP,
        stamp,
    ).map_err(|e| format!("Error storing last message client id timestamp: {}", e))?;
    return Ok(MessageClientId::from_timestamp(stamp));
}

#[derive(Serialize, Deserialize, Clone)]
pub enum OutboxMessageReplyTo {
    Channel(QualifiedMessageId),
    Outbox(MessageClientId),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum OutboxMessageRel {
    None,
    ReplyTo(OutboxMessageReplyTo),
    EditOf(QualifiedMessageId),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum OutboxFile {
    Local(String),
    Remote(QualifiedFileId),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OutboxBlockFile {
    pub file: OutboxFile,
    pub mime_type: String,
    pub size: u64,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum OutboxBlock {
    Text(String),
    File(OutboxBlockFile),
    Event(MessageBlockEvent),
    Extension(MessageBlockExtension),
}

#[derive(Serialize, Deserialize)]
pub struct OutboxMessage {
    pub rel: OutboxMessageRel,
    pub client_id: MessageClientId,
    pub blocks: Vec<OutboxBlock>,
}

pub async fn opfs_write_outbox_message(
    own_identity: &Identity,
    channel: &QualifiedChannelId,
    rel: OutboxMessageRel,
    client_id: MessageClientId,
    blocks: Vec<ViewBlock>,
) -> Result<(), String> {
    let opfs_channel_dir = opfs_outbox_channel_dir(own_identity, channel).await;
    let opfs_message_dir = opfs_outbox_message_dir(&opfs_channel_dir, &client_id).await;
    let mut out_blocks = vec![];
    for (i, block) in blocks.into_iter().enumerate() {
        match block {
            ViewBlock::Text(t) => out_blocks.push(OutboxBlock::Text(t)),
            ViewBlock::Extension(e) => out_blocks.push(OutboxBlock::Extension(e)),
            ViewBlock::Event(e) => out_blocks.push(OutboxBlock::Event(e)),
            ViewBlock::File(f) => {
                let file = match f.source {
                    ViewFileSource::Remote { channel: _, file } => OutboxFile::Remote(file),
                    ViewFileSource::Local(local) => {
                        let seg = format!("file{}", i);
                        let handle =
                            FileSystemFileHandle::from(
                                JsFuture::from(opfs_message_dir.get_file_handle_with_options(&seg, &{
                                    let x = FileSystemGetFileOptions::new();
                                    x.set_create(true);
                                    x
                                })).await.map_err(|e| format!("Error creating outbox file [{}]: {:?}", seg, e))?,
                            );
                        let w =
                            FileSystemWritableFileStream::from(
                                JsFuture::from(handle.create_writable())
                                    .await
                                    .map_err(
                                        |e| format!("Error opening outbox file [{}] for writing: {:?}", seg, e),
                                    )?,
                            );
                        JsFuture::from(
                            w
                                .write_with_blob(&local.blob)
                                .map_err(|e| format!("Error writing outbox file [{}]: {:?}", seg, e))?,
                        )
                            .await
                            .map_err(|e| format!("Error writing outbox file [{}] (2): {:?}", seg, e))?;
                        JsFuture::from(w.close())
                            .await
                            .map_err(|e| format!("Error closing outbox file [{}]: {:?}", seg, e))?;
                        OutboxFile::Local(seg)
                    },
                };
                out_blocks.push(OutboxBlock::File(OutboxBlockFile {
                    file: file,
                    mime_type: f.mime_type,
                    size: f.size,
                    name: f.name,
                }));
            },
        }
    }
    opfs_write_json(&opfs_message_dir, OPFS_FILENAME_MAIN, OutboxMessage {
        rel: rel,
        client_id: client_id,
        blocks: out_blocks,
    }).await.map_err(|e| format!("Error writing message to opfs: [{}]", e))?;
    trigger_push();
    return Ok(());
}

pub async fn opfs_outbox() -> FileSystemDirectoryHandle {
    let opfs = opfs_root().await;
    return opfs_ensure_dir(&opfs, "outbox").await;
}

pub async fn opfs_outbox_channel_dir(ident: &Identity, channel: &QualifiedChannelId) -> FileSystemDirectoryHandle {
    let opfs = opfs_outbox().await;

    // Sender
    let opfs = opfs_ensure_dir(&opfs, &ident.to_string()).await;

    // Dest
    let opfs = opfs_ensure_dir(&opfs, &channel.identity.to_string()).await;
    let opfs = opfs_ensure_dir(&opfs, &channel.channel.0.to_string()).await;
    return opfs;
}

pub async fn opfs_outbox_message_dir(
    parent: &FileSystemDirectoryHandle,
    client_id: &MessageClientId,
) -> FileSystemDirectoryHandle {
    return opfs_ensure_dir(parent, &client_id.0).await;
}

pub async fn opfs_channel_dir_entries(
    channel_dir: &FileSystemDirectoryHandle,
) -> BTreeMap<Timestamp, FileSystemDirectoryHandle> {
    let entries0 = opfs_list_dir(&channel_dir).await;
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

pub async fn opfs_delete_outbox_message(sender: &Identity, channel: &QualifiedChannelId, client_id: &MessageClientId) {
    let channel_dir = opfs_outbox_channel_dir(sender, channel).await;
    for (name, _) in opfs_list_dir(&channel_dir).await {
        if name == client_id.0 {
            opfs_delete(&channel_dir, &name).await;
        }
    }
}
