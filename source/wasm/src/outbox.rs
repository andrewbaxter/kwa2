use {
    crate::{
        opfs::{
            opfs_delete,
            opfs_ensure_dir,
            opfs_list_dir,
            opfs_root,
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
        MessageClientId,
        QualifiedChannelId,
        QualifiedMessageId,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        collections::BTreeMap,
        str::FromStr,
    },
    wasm_bindgen::JsCast,
    web_sys::FileSystemDirectoryHandle,
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
    LocalStorage::set(LOCALSTORAGE_LAST_CLIENT_ID_STAMP, stamp).map_err(
        |e| format!("Error storing last message client id timestamp: {}", e),
    )?;
    return Ok(MessageClientId::from_timestamp(stamp));
}

#[derive(Serialize, Deserialize, Clone)]
pub enum OutboxMessageReplyTo {
    Channel(QualifiedMessageId),
    Outbox(MessageClientId),
}

#[derive(Serialize, Deserialize)]
pub struct OutboxMessage {
    pub reply_to: Option<OutboxMessageReplyTo>,
    pub client_id: MessageClientId,
    pub body: String,
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
