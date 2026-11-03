use {
    flowcontrol::ta_return,
    futures::channel::oneshot,
    gloo::{
        timers::future::TimeoutFuture,
        utils::format::JsValueSerdeExt,
    },
    js_sys::{
        Array,
        Promise,
    },
    shared::interface::wire::{
        s2c,
        sw::{
            FromSw,
            FromSwNotification,
            FromSwReload,
        },
    },
    wasm_bindgen::{
        JsCast,
        JsValue,
        prelude::wasm_bindgen,
    },
    wasm_bindgen_futures::{
        JsFuture,
        future_to_promise,
        spawn_local,
    },
    web_sys::{
        Cache,
        Client,
        FetchEvent,
        Headers,
        NotificationOptions,
        PushEvent,
        RequestMode,
        ServiceWorkerGlobalScope,
        WindowClient,
        console,
    },
    std::{
        cell::RefCell,
        collections::HashMap,
    },
};

const CACHE: &str = "cache1";
const VERSIONED_PREFIX: &str = "v/";
const ETAG: &str = "ETag";
const CACHE_CONTROL: &str = "Cache-Control";
const RELOAD_DEBOUNCE_MS: u32 = 1000;

#[derive(Default)]
struct ClientRequests {
    pending: usize,
    stale: Vec<String>,
    generation: usize,
}

thread_local! {
    static CLIENT_REQUESTS: RefCell<HashMap<String, ClientRequests>> = RefCell::new(HashMap::new());
}

fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = FetchEvent)]
    type NavigationFetchEvent;
    #[wasm_bindgen(method, getter, js_name = resultingClientId)]
    fn resulting_client_id(this: &NavigationFetchEvent) -> String;
}

fn post_reload(client: &Client, reason: String) -> Result<(), String> {
    console::log_1(&JsValue::from(format!("Reloading client {}: {}", client.id(), reason)));
    return post_message(client, &FromSw::Reload(FromSwReload { reason: reason }));
}

fn post_message(client: &Client, message: &FromSw) -> Result<(), String> {
    client
        .post_message(
            &<JsValue as JsValueSerdeExt>::from_serde(
                message,
            ).expect("Error converting service worker message into js value"),
        )
        .map_err(|e| format!("Error posting message to client: {:?}", e.as_string()))?;
    return Ok(());
}

fn is_immutable(resp: &web_sys::Response) -> bool {
    return resp
        .headers()
        .get(CACHE_CONTROL)
        .expect("Error checking for cache control header")
        .map(|directive| directive.contains("immutable"))
        .unwrap_or(false);
}

async fn evict_other_versions(cache: &Cache, base_url: &str, url: &str) -> Result<(), JsValue> {
    let versioned_base = format!("{}{}", base_url, VERSIONED_PREFIX);
    let Some(rest) = url.strip_prefix(&versioned_base) else {
        return Ok(());
    };
    let keep = format!("{}{}/", versioned_base, rest.split('/').next().unwrap());
    for key in Array::from(&JsFuture::from(cache.keys()).await?) {
        let key = key.dyn_into::<web_sys::Request>()?;
        let key_url = key.url();
        if key_url.starts_with(&versioned_base) && !key_url.starts_with(&keep) {
            JsFuture::from(cache.delete_with_request(&key)).await?;
        }
    }
    return Ok(());
}

fn sw_global() -> ServiceWorkerGlobalScope {
    return js_sys::global()
        .dyn_into::<ServiceWorkerGlobalScope>()
        .expect("Serviceworker self is not expected type");
}

#[wasm_bindgen]
pub fn handle_install() -> Result<Promise, JsValue> {
    return sw_global().skip_waiting();
}

#[wasm_bindgen]
pub fn handle_activate() -> Promise {
    let self_ = sw_global();
    return future_to_promise(async move {
        let caches = self_.caches()?;
        for k in Array::from(&JsFuture::from(caches.keys()).await?) {
            let Some(k) = k.as_string() else {
                console::log_2(&JsValue::from("Caches key is not a string, skipping"), &k);
                continue;
            };
            if k != CACHE {
                JsFuture::from(caches.delete(&k)).await?;
            }
        }
        let clients = self_.clients();
        let stale_clients = Array::from(&JsFuture::from(clients.match_all()).await?);
        JsFuture::from(clients.claim()).await?;
        for client in stale_clients {
            if let Err(e) = post_reload(&Client::from(client), format!("Service worker updated")) {
                console::log_1(&JsValue::from(format!("Error reloading client after service worker update: {}", e)));
            }
        }
        return Ok(JsValue::null());
    });
}

fn track_client_request(self_: &ServiceWorkerGlobalScope, ev: &FetchEvent) -> Option<oneshot::Sender<String>> {
    let client_id = match ev.client_id().filter(|c| !c.is_empty()) {
        Some(c) => c,
        None => Some(ev.unchecked_ref::<NavigationFetchEvent>().resulting_client_id()).filter(|c| !c.is_empty())?,
    };
    CLIENT_REQUESTS.with_borrow_mut(|clients| {
        let client = clients.entry(client_id.clone()).or_default();
        client.pending += 1;
        client.generation += 1;
    });
    let (stale_tx, stale_rx) = oneshot::channel::<String>();
    let self_ = self_.clone();
    ev.wait_until(&future_to_promise(async move {
        let stale = stale_rx.await.ok();
        finish_client_request(self_, client_id, stale).await;
        return Ok(JsValue::UNDEFINED);
    })).expect("Fetch event no longer active while handling fetch");
    return Some(stale_tx);
}

async fn finish_client_request(self_: ServiceWorkerGlobalScope, client_id: String, stale: Option<String>) {
    let Some(generation) = CLIENT_REQUESTS.with_borrow_mut(|clients| {
        let client = clients.get_mut(&client_id).unwrap();
        client.pending -= 1;
        client.stale.extend(stale);
        if client.pending > 0 {
            return None;
        }
        if client.stale.is_empty() {
            clients.remove(&client_id);
            return None;
        }
        return Some(client.generation);
    }) else {
        return;
    };
    TimeoutFuture::new(RELOAD_DEBOUNCE_MS).await;
    let Some(stale) = CLIENT_REQUESTS.with_borrow_mut(|clients| {
        if clients.get(&client_id).unwrap().generation != generation {
            return None;
        }
        return Some(clients.remove(&client_id).unwrap().stale);
    }) else {
        return;
    };
    match async {
        ta_return!((), String);
        let Ok(client) =
            JsFuture::from(self_.clients().get(&client_id))
                .await
                .map_err(|e| format!("Error looking up client {}: {:?}", client_id, e.as_string()))?
                .dyn_into::<Client>() else {
                return Ok(());
            };
        post_reload(&client, format!("Cached responses were stale: {}", stale.join(", ")))?;
        return Ok(());
    }.await {
        Ok(_) => { },
        Err(e) => {
            console::log_1(&JsValue::from(format!("Error triggering client reload due to stale cached responses: {}", e)));
        },
    }
}

#[wasm_bindgen]
pub fn handle_fetch(ev: FetchEvent) -> Promise {
    let self_ = sw_global();
    let base_url = format!("{}/", self_.location().origin());
    let stale_tx = track_client_request(&self_, &ev);
    let req = ev.request();
    return future_to_promise(async move {
        let url = req.url();

        // Not cacheable, send as usual
        if !url.starts_with(&base_url) || req.method() != "GET" {
            return JsFuture::from(self_.fetch_with_request(&req)).await;
        }

        // Look for cached result
        let caches = self_.caches()?;
        let cache_store = Cache::from(JsFuture::from(caches.open(CACHE)).await?);
        let cached_resp = JsFuture::from(cache_store.match_with_request(&req)).await?;
        let cache = {
            let cache = cache_store.clone();
            async move |req: &web_sys::Request, resp: &web_sys::Response| -> bool {
                let want_cache =
                    resp.headers().get(ETAG).expect("Error checking for etag header").is_some() || is_immutable(resp);
                if !want_cache {
                    return false;
                }
                let resp = match resp.clone() {
                    Ok(r) => r,
                    Err(e) => {
                        console::log_2(&JsValue::from("Error cloning response for caching"), &e);
                        return false;
                    },
                };
                if let Err(e) = JsFuture::from(cache.put_with_request(&req, &resp)).await {
                    console::log_2(&JsValue::from("Error caching response"), &e);
                    return false;
                }
                return true;
            }
        };

        // No cached result, send as usual and try caching
        if cached_resp.is_undefined() {
            let resp = JsFuture::from(self_.fetch_with_request(&req)).await?;
            let resp = resp.dyn_into::<web_sys::Response>()?;
            if cache(&req, &resp).await {
                if let Err(e) = evict_other_versions(&cache_store, &base_url, &url).await {
                    console::log_2(&JsValue::from("Error evicting other app versions from cache"), &e);
                }
            }
            return Ok(resp.into());
        }
        let cached_resp = cached_resp.dyn_into::<web_sys::Response>().unwrap();

        if is_immutable(&cached_resp) {
            return Ok(cached_resp.into());
        }

        // If cached due to etag
        if let Some(cached_etag_value) = cached_resp.headers().get(ETAG)? {
            // If application file, optimistically send cached result; in the bg re-send the
            // request and reload if it changed
            spawn_local(async move {
                let req_with_if_none_match = |req: &web_sys::Request| -> Result<web_sys::Request, JsValue> {
                    let o = web_sys::RequestInit::new();
                    let headers = Headers::new_with_headers(&req.headers())?;
                    headers.set("If-None-Match", &cached_etag_value)?;
                    o.set_headers(&headers.into());
                    o.set_mode(RequestMode::SameOrigin);
                    return web_sys::Request::new_with_str_and_init(&req.url(), &o);
                };
                let revalidate_req = match req_with_if_none_match(&req) {
                    Ok(r) => r,
                    Err(e) => {
                        console::log_2(&JsValue::from("Error building optimistic etag request"), &e);
                        return;
                    },
                };
                let resp = match JsFuture::from(self_.fetch_with_request(&revalidate_req)).await {
                    Ok(v) => v,
                    Err(e) => {
                        console::log_1(
                            &JsValue::from(format!("Optimistic etag request failed: {:?}", e.as_string())),
                        );
                        return;
                    },
                }.dyn_into::<web_sys::Response>().unwrap();
                if resp.status() == 200 {
                    if !cache(&req, &resp).await {
                        console::log_1(
                            &JsValue::from(format!("Revalidated response for {} changed but wasn't cached, not reloading", url)),
                        );
                        return;
                    }
                    if let Some(stale_tx) = stale_tx {
                        _ = stale_tx.send(format!("{} (ETag {} -> {})", url, cached_etag_value, match resp.headers().get(ETAG) {
                            Ok(Some(v)) => v,
                            Ok(None) => format!("none"),
                            Err(e) => format!("error reading: {:?}", e.as_string()),
                        }));
                    }
                }
            });

            // Optimistically return cached response
            return Ok(cached_resp.into());
        }

        unreachable!();
    });
}

#[wasm_bindgen]
pub fn handle_push(ev: PushEvent) -> Promise {
    // Post to visible clients, or else show a notification if there are none
    let self_ = sw_global();
    return future_to_promise(async move {
        match async {
            ta_return!((), String);
            let Some(data) = ev.data() else {
                return Err(format!("Push event has no data"));
            };
            let data =
                data
                    .json()
                    .map_err(|e| format!("Error reading data from push event as json: {:?}", e.as_string()))?;
            let data: s2c::Notification =
                JsValueSerdeExt::into_serde(
                    &data,
                ).map_err(|e| format!("Received payload doesn't match expected format: {}", e))?;
            let mut visible_clients = false;
            for client in Array::from(
                &JsFuture::from(self_.clients().match_all())
                    .await
                    .map_err(|e| format!("Error listing clients: {:?}", e.as_string()))?,
            ) {
                let client = WindowClient::from(client);
                match client.visibility_state() {
                    web_sys::VisibilityState::Visible => {
                        if let Err(e) = post_message(&client, &FromSw::Notification(FromSwNotification {
                            channel: data.channel.clone(),
                            offset: data.offset,
                        })) {
                            console::log_1(
                                &JsValue::from(format!("Error forwarding push notification to visible client: {}", e)),
                            );
                        }
                        visible_clients = true;
                    },
                    _ => { },
                }
            }
            if !visible_clients {
                JsFuture::from(self_.registration().show_notification_with_options(&data.body, &{
                    let o = NotificationOptions::new();
                    o
                }).map_err(|e| format!("Error showing notification: {:?}", e.as_string()))?)
                    .await
                    .map_err(|e| format!("Error showing notification (async): {:?}", e.as_string()))?;
            }
            return Ok(());
        }.await {
            Ok(_) => { },
            Err(e) => {
                console::log_1(&JsValue::from(format!("Error handling push message: {}", e)));
            },
        }
        return Ok(JsValue::null());
    });
}
