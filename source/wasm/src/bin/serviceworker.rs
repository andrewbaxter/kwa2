use {
    flowcontrol::ta_return,
    gloo::{
        events::EventListener,
        utils::{
            format::JsValueSerdeExt,
            window,
        },
    },
    js_sys::{
        Array,
        AsyncIterator,
        Reflect,
    },
    rooting::set_root_non_dom,
    shared::interface::{
        wire::c2s,
        PATH_PREFIX_CLIENT,
    },
    tokio_stream::StreamExt,
    wasm::swproto::FromSw,
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
    wasm_bindgen_futures::{
        future_to_promise,
        spawn_local,
        stream::JsStream,
        JsFuture,
    },
    web_sys::{
        console,
        Cache,
        ExtendableEvent,
        FetchEvent,
        Headers,
        NotificationOptions,
        PushEvent,
        RequestMode,
        ServiceWorkerGlobalScope,
        WindowClient,
    },
};

const CACHE: &str = "cache1";

fn main() {
    let self_ =
        Reflect::get(&js_sys::global(), &JsValue::from_str("WorkerGlobalScope"))
            .expect("Error getting serviceworker self")
            .dyn_into::<ServiceWorkerGlobalScope>()
            .expect("Serviceworker self is not expected type");
    let mut root = vec![];
    root.push(EventListener::new(&self_, "install", {
        let self_ = self_.clone();
        move |_ev| {
            let f1 = self_.skip_waiting().expect("Error skipping waiting for service worker installation");
            spawn_local(async move {
                JsFuture::from(f1).await.expect("Error completing skip_waiting call");
            });
        }
    }));
    root.push(EventListener::new(&self_, "activate", |ev| {
        ev.dyn_ref::<ExtendableEvent>().unwrap().wait_until(&future_to_promise(async move {
            let caches = window().caches().expect("Error retrieving caches object");
            let mut keys = JsStream::from(caches.keys().dyn_into::<AsyncIterator>().unwrap());
            while let Some(k) = keys.next().await {
                let k = match k {
                    Ok(k) => k,
                    Err(e) => {
                        panic!("Error retrieving key from caches: {:?}", e.as_string());
                    },
                };
                let k = k.as_string().expect("Caches key is not a string");
                if k != CACHE {
                    JsFuture::from(caches.delete(&k)).await.expect("Failed to delete caches key");
                }
            }
            return Ok(JsValue::null());
        })).expect("Error waiting for caches cleanup");
    }));
    root.push(EventListener::new(&self_, "fetch", {
        let self_ = self_.clone();
        let base_url = self_.location().origin();
        move |ev| {
            let ev = ev.dyn_ref::<FetchEvent>().unwrap();
            let client_id = ev.client_id();
            let req = ev.request();
            let self_ = self_.clone();
            let base_url = base_url.clone();
            if let Err(e) = ev.respond_with(&future_to_promise(async move {
                let url = req.url();

                // Not cacheable, send as usual
                if url.starts_with(&base_url) || req.method() != "GET" {
                    return JsFuture::from(self_.fetch_with_request(&req)).await;
                }

                // Look for cached result
                const ETAG: &str = "ETag";
                const CACHE_CONTROL: &str = "Cache-Control";
                let caches = window().caches().expect("Error retrieving caches object");
                let cache = Cache::from(JsFuture::from(caches.open(CACHE)).await.expect("Error opening cache"));
                let cached_resp =
                    JsFuture::from(cache.match_with_request(&req))
                        .await
                        .expect("Error looking up request in cache");
                let cache = async move |req: &web_sys::Request, resp: &web_sys::Response| {
                    if let Some(_) = resp.headers().get(ETAG).expect("Error checking for etag header") {
                        if let Err(e) = JsFuture::from(cache.put_with_request(&req, resp)).await {
                            console::log_2(&JsValue::from("Error caching response"), &e);
                        }
                    }
                    if let Some(directive) =
                        resp.headers().get(CACHE_CONTROL).expect("Error checking for cache control header") {
                        if directive.contains("immutable") {
                            if let Err(e) = JsFuture::from(cache.put_with_request(&req, resp)).await {
                                console::log_2(&JsValue::from("Error caching response"), &e);
                            }
                        }
                    }
                };

                // No cached result, send as usual and try caching
                if cached_resp.is_null() {
                    let resp = JsFuture::from(self_.fetch_with_request(&req)).await?;
                    let resp = resp.dyn_into::<web_sys::Response>()?;
                    cache(&req, &resp).await;
                    return Ok(resp.into());
                }
                let cached_resp = cached_resp.dyn_into::<web_sys::Response>().unwrap();

                // If cached due to etag
                if let Some(cached_etag_value) = cached_resp.headers().get(ETAG)? {
                    let req_with_if_none_match = move |req: &web_sys::Request| -> web_sys::Request {
                        return web_sys::Request::new_with_request_and_init(req, &{
                            let o = web_sys::RequestInit::new();
                            let headers =
                                Headers::new_with_headers(
                                    &req.headers(),
                                ).expect("Error creating copy of request headers");
                            headers
                                .set("If-None-Match", &cached_etag_value)
                                .expect("Error setting if-none-match header");
                            o.set_headers(&headers.into());
                            o.set_mode(RequestMode::SameOrigin);
                            o
                        }).expect("Error creating request copy with if-none-match");
                    };
                    if url.contains(PATH_PREFIX_CLIENT) {
                        // If application file, optimistically send cached result; in the bg re-send the
                        // request and reload if it changed
                        spawn_local(async move {
                            let resp =
                                match JsFuture::from(
                                    self_.fetch_with_request(&req_with_if_none_match(&req)),
                                ).await {
                                    Ok(v) => v,
                                    Err(e) => {
                                        console::log_1(
                                            &JsValue::from(
                                                format!("Optimistic etag request failed: {:?}", e.as_string()),
                                            ),
                                        );
                                        return;
                                    },
                                }.dyn_into::<web_sys::Response>().unwrap();
                            if resp.status() == 200 {
                                // Changed, cache
                                cache(&req, &resp).await;

                                // Tell client to reload to use newer data
                                match async {
                                    ta_return!((), String);
                                    let Some(client_id) = client_id else {
                                        return Ok(());
                                    };
                                    let Ok(client) =
                                        JsFuture::from(self_.clients().get(&client_id))
                                            .await
                                            .map_err(
                                                |e| format!(
                                                    "Error looking up client {}: {:?}",
                                                    client_id,
                                                    e.as_string()
                                                ),
                                            )?
                                            .dyn_into::<WindowClient>() else {
                                            return Ok(());
                                        };
                                    client
                                        .post_message(
                                            &<JsValue as JsValueSerdeExt>::from_serde(
                                                &FromSw::Reload,
                                            ).map_err(
                                                |e| format!(
                                                    "Error converting service worker message to js value: {:?}",
                                                    e
                                                ),
                                            )?,
                                        )
                                        .map_err(
                                            |e| format!(
                                                "Error sending message from service worker to client: {:?}",
                                                e.as_string()
                                            ),
                                        )?;
                                    return Ok(());
                                }.await {
                                    Ok(_) => { },
                                    Err(e) => {
                                        console::log_1(
                                            &JsValue::from(
                                                format!(
                                                    "Error triggering client refresh due to optimistic request stale result: {}",
                                                    e
                                                ),
                                            ),
                                        );
                                    },
                                }
                            }
                        });

                        // Optimistically return cached response
                        return Ok(cached_resp.into());
                    } else {
                        // Non-application file; cautiously send with if-none-match and wait before
                        // responding
                        let resp =
                            JsFuture::from(self_.fetch_with_request(&req_with_if_none_match(&req)))
                                .await?
                                .dyn_into::<web_sys::Response>()
                                .unwrap();
                        if resp.status() == 200 {
                            cache(&req, &resp).await;
                            return Ok(resp.into());
                        } else {
                            return Ok(cached_resp.into());
                        }
                    }
                }

                // If cached due to cache-control (only if immutable), respond with cached response
                if let Some(_) = cached_resp.headers().get(CACHE_CONTROL)? {
                    return Ok(cached_resp.into());
                }

                // All cached responses should be cache or etag
                unreachable!();
            })) {
                console::log_2(&JsValue::from("Fetch event handler exited with error"), &e);
            };
        }
    }));
    root.push(EventListener::new(&self_, "push", {
        let self_ = self_.clone();
        move |ev| {
            // Post to visible clients, or else show a notification if there are none
            let ev = ev.dyn_ref::<PushEvent>().unwrap();
            if let Err(e) = ev.wait_until(&future_to_promise({
                let ev = ev.clone();
                let self_ = self_.clone();
                async move {
                    match async {
                        ta_return!((), String);
                        let data =
                            ev
                                .data()
                                .expect("Error reading data from push event")
                                .json()
                                .expect("Error reading data from push event as json");
                        let data: c2s::Notification =
                            JsValueSerdeExt::into_serde(
                                &data,
                            ).expect("Received payload doesn't match expected format");
                        let mut visible_clients = false;
                        for client in Array::from(
                            &JsFuture::from(self_.clients().match_all()).await.expect("Error listing clients"),
                        ) {
                            let client = WindowClient::from(client);
                            match client.visibility_state() {
                                web_sys::VisibilityState::Visible => {
                                    if let Err(e) =
                                        client.post_message(
                                            &<JsValue as JsValueSerdeExt>::from_serde(
                                                &FromSw::Notification(data.clone()),
                                            ).expect("Error converting notification message into js value"),
                                        ) {
                                        console::log_2(
                                            &JsValue::from("Error forwarding push notification to visible client"),
                                            &e,
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
                }
            })) {
                console::log_2(&JsValue::from("Push event handler exited with error"), &e);
            };
        }
    }));
    set_root_non_dom(root);
}
