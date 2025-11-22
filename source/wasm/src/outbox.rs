use {
    crate::state::state,
    gloo::utils::window,
    jiff::Timestamp,
    js_sys::Array,
    serde::{
        Deserialize,
        Serialize,
        de::DeserializeOwned,
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
    tokio_stream::StreamExt,
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
    wasm_bindgen_futures::{
        JsFuture,
        stream::JsStream,
    },
    web_sys::{
        FileSystemDirectoryHandle,
        FileSystemFileHandle,
        FileSystemGetDirectoryOptions,
        FileSystemWritableFileStream,
    },
};

pub const OPFS_FILENAME_MAIN: &str = "main";

#[derive(Serialize, Deserialize)]
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

pub async fn opfs_list_dir(parent: &FileSystemDirectoryHandle) -> Vec<(String, JsValue)> {
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

pub async fn opfs_read_json<
    T: DeserializeOwned,
>(parent: &FileSystemDirectoryHandle, seg: &str) -> Result<T, String> {
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

pub async fn opfs_write_json<
    T: Serialize,
>(parent: &FileSystemDirectoryHandle, seg: &str, data: T) -> Result<(), String> {
    let f =
        FileSystemFileHandle::from(
            JsFuture::from(parent.get_file_handle(seg))
                .await
                .map_err(|e| format!("Error getting file handle at seg [{}]: {:?}", seg, e.as_string()))?,
        );
    let w =
        FileSystemWritableFileStream::from(
            JsFuture::from(f.create_writable())
                .await
                .map_err(|e| format!("Error getting file handle writable at seg [{}]: {:?}", seg, e.as_string()))?,
        );
    JsFuture::from(
        w
            .write_with_str(&serde_json::to_string(&data).unwrap())
            .map_err(|e| format!("Error writing message to opfs file at seg [{}]: {:?}", seg, e.as_string()))?,
    )
        .await
        .map_err(|e| format!("Error writing message to opfs file at seg [{}] (2): {:?}", seg, e.as_string()))?;
    return Ok(());
}

pub async fn opfs_delete(parent: &FileSystemDirectoryHandle, seg: &str) {
    if let Err(e) = JsFuture::from(parent.remove_entry(seg)).await {
        state().log.log_js(&format!("Error deleting opfs entry at [{}]", seg), &e);
    }
}

pub async fn opfs_outbox() -> FileSystemDirectoryHandle {
    let opfs =
        JsFuture::from(window().navigator().storage().get_directory())
            .await
            .expect("Error getting opfs root")
            .dyn_into::<FileSystemDirectoryHandle>()
            .unwrap();
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
