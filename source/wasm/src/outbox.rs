use {
    crate::state::state,
    gloo::utils::window,
    jiff::Timestamp,
    js_sys::Array,
    serde::{
        de::DeserializeOwned,
        Deserialize,
        Serialize,
    },
    shared::interface::shared::{
        MessageIdem,
        QualifiedChannelId,
    },
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
        stream::JsStream,
        JsFuture,
    },
    web_sys::{
        FileSystemDirectoryHandle,
        FileSystemFileHandle,
        FileSystemGetDirectoryOptions,
    },
};

pub const OPFS_FILENAME_MAIN: &str = "main";

#[derive(Serialize, Deserialize)]
pub struct OutboxMessage {
    pub idem: MessageIdem,
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

pub async fn opfs_channel_dir(channel: &QualifiedChannelId) -> FileSystemDirectoryHandle {
    let opfs =
        JsFuture::from(window().navigator().storage().get_directory())
            .await
            .expect("Error getting opfs root")
            .dyn_into::<FileSystemDirectoryHandle>()
            .unwrap();
    let opfs = opfs_ensure_dir(&opfs, "outbox").await;
    let opfs = opfs_ensure_dir(&opfs, &channel.identity.to_string()).await;
    let opfs = opfs_ensure_dir(&opfs, &channel.channel.0.to_string()).await;
    return opfs;
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
