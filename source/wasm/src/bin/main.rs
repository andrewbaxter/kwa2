use {
    flowcontrol::shed,
    gloo::{
        events::EventListener,
        storage::{
            LocalStorage,
            SessionStorage,
            Storage,
        },
        utils::{
            document,
            format::JsValueSerdeExt,
            window,
        },
    },
    lunk::EventGraph,
    rooting::set_root,
    serde::Deserialize,
    std::{
        cell::RefCell,
        panic,
        rc::Rc,
    },
    wasm::{
        async_::bg_val,
        chat_entry::ChatEntryLookup,
        js::{
            scan_env,
            style_export::{
                self,
            },
            Log,
            LogJsErr,
            VecLog,
        },
        state::{
            build_ministate,
            read_ministate,
            record_replace_ministate,
            state,
            Ministate,
            State_,
            LOCALSTORAGE_PWA_MINISTATE,
            SESSIONSTORAGE_POST_REDIRECT_MINISTATE,
            STATE,
        },
        serviceworker_proto::FromSw,
    },
    wasm_bindgen::{
        JsCast,
        JsValue,
    },
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        MessageEvent,
        ServiceWorkerRegistration,
    },
};

pub fn main() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let eg = EventGraph::new();
    let log1 = Rc::new(VecLog { log: Default::default() });
    let log = log1.clone() as Rc<dyn Log>;
    eg.event(|pc| {
        let env = scan_env(&log);
        let service_worker = bg_val(async {
            let sw =
                JsFuture::from(window().navigator().service_worker().register("./serviceworker.js"))
                    .await
                    .map_err(|e| format!("Error registering service worker: {:?}", e.as_string()))?;
            return Ok(sw.dyn_into::<ServiceWorkerRegistration>().unwrap());
        });
        let root = style_export::cont_group(style_export::ContGroupArgs { children: vec![] }).root;

        // Build app state
        STATE.with(|s| *s.borrow_mut() = Some(Rc::new(State_ {
            eg: pc.eg(),
            current_chat: Default::default(),
            channel_feeds: Default::default(),
            outbox_entries: ChatEntryLookup::new(),
            service_worker: service_worker,
            root: root.clone(),
            ministate: RefCell::new(shed!{
                'found _;
                shed!{
                    let m = match SessionStorage::get::<Ministate>(SESSIONSTORAGE_POST_REDIRECT_MINISTATE) {
                        Ok(m) => m,
                        Err(e) => match e {
                            gloo::storage::errors::StorageError::KeyNotFound(_) => {
                                break;
                            },
                            gloo::storage::errors::StorageError::SerdeError(..) |
                            gloo::storage::errors::StorageError::JsError(..) => {
                                log.log(
                                    &format!("Error reading post-redirect ministate from session storage: {}", e),
                                );
                                break;
                            },
                        },
                    };
                    SessionStorage::delete(SESSIONSTORAGE_POST_REDIRECT_MINISTATE);
                    record_replace_ministate(&log, &m);
                    break 'found m;
                }
                shed!{
                    if !env.pwa {
                        break;
                    }
                    let m = match LocalStorage::get::<Ministate>(LOCALSTORAGE_PWA_MINISTATE) {
                        Ok(m) => m,
                        Err(e) => match e {
                            gloo::storage::errors::StorageError::KeyNotFound(_) => {
                                break;
                            },
                            gloo::storage::errors::StorageError::SerdeError(..) |
                            gloo::storage::errors::StorageError::JsError(..) => {
                                log.log(&format!("Error reading pwa ministate from local storage: {}", e));
                                break;
                            },
                        },
                    };
                    record_replace_ministate(&log, &m);
                }
                break 'found read_ministate(&log);
            }),
            env: env.clone(),
            log1: log1,
            log: log.clone(),
        })));

        // Load initial view
        build_ministate(pc, &state().ministate.borrow());

        // React to further state changes
        EventListener::new(&window(), "popstate", {
            let eg = pc.eg();
            move |_e| eg.event(|pc| {
                let ministate = read_ministate(&state().log);
                *state().ministate.borrow_mut() = ministate.clone();
                LocalStorage::set(
                    LOCALSTORAGE_PWA_MINISTATE,
                    &ministate,
                ).log(&state().log, &"Error storing PWA state");
                build_ministate(pc, &ministate);
            }).unwrap()
        }).forget();
        EventListener::new(&window().navigator().service_worker(), "message", {
            let eg = pc.eg();
            move |ev| eg.event(|pc| {
                let ev =
                    ev
                        .dyn_ref::<MessageEvent>()
                        .expect("Got wrong event type in service worker message event handler");
                let data =
                    <JsValue as JsValueSerdeExt>::into_serde::<FromSw>(
                        &ev.data(),
                    ).expect("Got wrong data type from service worker");
                match data {
                    FromSw::Reload => {
                        document().location().unwrap().reload().expect("Error triggering reload");
                    },
                    FromSw::Notification(n) => {
                        let state = state();
                        let cf = state.channel_feeds.borrow();
                        if let Some(f) = cf.get(&n.channel) {
                            f.channel.notify(pc, n.offset);
                        }
                    },
                }
                ev.data();
            }).unwrap()
        }).forget();

        // Root and display
        set_root(vec![root.own(|_| (
            //. .
            EventListener::new(&window(), "message", |ev| {
                let ev = ev.dyn_ref::<MessageEvent>().unwrap();

                #[derive(Deserialize)]
                #[serde(rename_all = "snake_case", deny_unknown_fields)]
                enum Message {
                    Log(String),
                    Reload,
                }

                let message = match JsValueSerdeExt::into_serde::<Message>(&ev.data()) {
                    Ok(m) => m,
                    Err(e) => {
                        state().log.log(&format!("Error parsing js message: {}", e));
                        return;
                    },
                };
                match message {
                    Message::Log(m) => {
                        state().log.log(&format!("From service worker: {}", m));
                    },
                    Message::Reload => {
                        window()
                            .location()
                            .reload()
                            .log(&state().log, &"Error executing reload triggered by web worker.");
                    },
                }
            }),
        ))]);
    });
}
