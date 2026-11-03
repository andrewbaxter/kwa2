use {
    crate::{
        icon_gen::{
            channel_icon,
            identity_icon,
        },
        js::LogJsErr,
        state::{
            Ministate,
            SESSIONSTORAGE_POST_REDIRECT_MINISTATE,
            state,
        },
    },
    gloo::{
        storage::{
            SessionStorage,
            Storage,
        },
        timers::future::TimeoutFuture,
        utils::window,
    },
    reqwasm::http::{
        Request,
        Response,
    },
    shared::interface::{
        shared::{
            FileRef,
            QualifiedChannelId,
        },
        wire::{
            c2s::{
                self,
                GetResp,
                PathReqTrait,
            },
            kwaurl::KwaUrl,
        },
    },
    spaghettinuum::interface::identity::Identity,
    wasm_bindgen::UnwrapThrowExt,
    web_sys::Url,
};

pub fn redirect_login() -> ! {
    if !SessionStorage::get::<Ministate>(SESSIONSTORAGE_POST_REDIRECT_MINISTATE).is_ok() {
        SessionStorage::set(
            SESSIONSTORAGE_POST_REDIRECT_MINISTATE,
            &*state().ministate.borrow(),
        ).log(&state().log, &"Error storing post-redirect ministate");
    }
    let base_url = &state().env.base_url;
    window()
        .location()
        .set_href(
            &Url::new_with_base("oidc", base_url).unwrap().href(),
        )
        .unwrap();
    unreachable!();
}

pub fn redirect_logout() -> ! {
    if !SessionStorage::get::<Ministate>(SESSIONSTORAGE_POST_REDIRECT_MINISTATE).is_ok() {
        SessionStorage::set(
            SESSIONSTORAGE_POST_REDIRECT_MINISTATE,
            &*state().ministate.borrow(),
        ).log(&state().log, &"Error storing post-redirect ministate");
    }
    let base_url = &state().env.base_url;
    window()
        .location()
        .set_href(
            &Url::new_with_base("logout", base_url).unwrap().href(),
        )
        .unwrap();
    unreachable!();
}

async fn read_resp(resp: Response) -> Result<Vec<u8>, String> {
    let status = resp.status();
    if status == 401 {
        redirect_login();
    }
    let body = match resp.binary().await {
        Err(e) => {
            return Err(format!("Got error response, got additional error trying to read body [{}]: {}", status, e));
        },
        Ok(r) => r,
    };
    if status >= 400 {
        return Err(format!("Got error response [{}]: [{}]", status, String::from_utf8_lossy(&body)));
    }
    return Ok(body);
}

async fn post(req: Request) -> Result<Vec<u8>, String> {
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(format!("Failed to send request: {}", e));
        },
    };
    return read_resp(resp).await;
}

pub async fn req_post_json<T: c2s::proto::ReqTrait>(base_url: &str, req: T) -> Result<T::Resp, String> {
    let req =
        Request::post(&format!("{}api", base_url))
            .header("Content-type", "application/json")
            .body(serde_json::to_string(&req.to_enum()).unwrap_throw());
    let body = post(req).await?;
    return Ok(
        serde_json::from_slice::<T::Resp>(
            &body,
        ).map_err(
            |e| format!("Error parsing JSON response from server: {}\nBody: {}", e, String::from_utf8_lossy(&body)),
        )?,
    );
}

pub async fn req_file(url: &str) -> Result<Vec<u8>, String> {
    let resp = match Request::get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(format!("Failed to send request: {}", e));
        },
    };
    return read_resp(resp).await;
}

pub async fn req_file_blob(url: &str, mime_type: &str) -> Result<web_sys::Blob, String> {
    let body = req_file(url).await?;
    return Ok(gloo::file::Blob::new_with_options(body.as_slice(), Some(mime_type)).into());
}

pub enum GetFailure {
    Transient(String),
    Permanent(String),
}

async fn get_once<T: c2s::PathReqTrait>(req: &T) -> Result<T::Resp, GetFailure> {
    let resp = match Request::get(&format!("{}{}", state().env.base_url, req.serialize_path())).send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(GetFailure::Transient(format!("Failed to send request: {}", e)));
        },
    };
    let status = resp.status();
    let body = match read_resp(resp).await {
        Ok(b) => b,
        Err(e) => {
            if status >= 500 {
                return Err(GetFailure::Transient(e));
            } else {
                return Err(GetFailure::Permanent(e));
            }
        },
    };
    match serde_json::from_slice::<GetResp<T::Resp>>(
        &body,
    ).map_err(
        |e| GetFailure::Permanent(
            format!(
                "Error parsing JSON response from server response: {}\nBody: {}",
                e,
                String::from_utf8_lossy(&body)
            ),
        ),
    )? {
        GetResp::Ok(v) => return Ok(v),
        GetResp::OwnerUnreachable => return Err(GetFailure::Transient(format!("Channel owner unreachable"))),
    }
}

pub async fn req_get<T: c2s::PathReqTrait>(req: T) -> Result<T::Resp, String> {
    match get_once(&req).await {
        Ok(v) => return Ok(v),
        Err(GetFailure::Transient(e)) | Err(GetFailure::Permanent(e)) => return Err(e),
    }
}

const GET_RETRY_INITIAL_MS: u32 = 1000;
const GET_RETRY_MAX_MS: u32 = 60 * 1000;

pub async fn req_get_retry<T: c2s::PathReqTrait>(req: T) -> Result<T::Resp, String> {
    let mut backoff = GET_RETRY_INITIAL_MS;
    loop {
        match get_once(&req).await {
            Ok(v) => return Ok(v),
            Err(GetFailure::Permanent(e)) => return Err(e),
            Err(GetFailure::Transient(_)) => {
                TimeoutFuture::new(backoff).await;
                backoff = (backoff * 2).min(GET_RETRY_MAX_MS);
            },
        }
    }
}

pub fn file_url(file: &FileRef) -> String {
    return format!("{}{}", state().env.base_url, c2s::GetFile { hash: file.hash.clone() }.serialize_path());
}

pub struct MenuImage {
    pub image: Option<String>,
    pub generated_image: Option<String>,
}

pub fn channel_image(id: &QualifiedChannelId, portrait: &Option<FileRef>) -> MenuImage {
    match portrait {
        Some(p) => return MenuImage {
            image: Some(file_url(p)),
            generated_image: None,
        },
        None => return MenuImage {
            image: None,
            generated_image: Some(channel_icon(id)),
        },
    }
}

pub fn identity_image(id: &Identity, portrait: &Option<FileRef>) -> MenuImage {
    match portrait {
        Some(p) => return MenuImage {
            image: Some(file_url(p)),
            generated_image: None,
        },
        None => return MenuImage {
            image: None,
            generated_image: Some(identity_icon(id)),
        },
    }
}

pub fn join_preview_file_url(url: &KwaUrl, file: &FileRef) -> String {
    return format!("{}{}", state().env.base_url, c2s::GetJoinPreviewFile {
        url: url.clone(),
        hash: file.hash.clone(),
    }.serialize_path());
}
