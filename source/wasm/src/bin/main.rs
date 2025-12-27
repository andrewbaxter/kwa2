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
    shared::interface::{
        shared::QualifiedChannelId,
        wire::{
            c2s::ActivityOffset,
            s2c,
        },
    },
    std::{
        cell::RefCell,
        panic,
        rc::Rc,
    },
    wasm::{
        async_::bg_val,
        background::{
            schedule_trigger_pull,
            trigger_push,
        },
        js::{
            Log,
            LogJsErr,
            VecLog,
            scan_env,
            style_export::{
                self,
            },
        },
        localdata::{
            get_stored_api_channelgroups,
            get_stored_api_channels,
        },
        page_top,
        serviceworker_proto::FromSw,
        state::{
            LOCALSTORAGE_PWA_MINISTATE,
            Ministate,
            SESSIONSTORAGE_POST_REDIRECT_MINISTATE,
            STATE,
            State_,
            build_ministate,
            get_setting_wide_view,
            read_ministate,
            record_replace_ministate,
            state,
            merge_top,
        },
        websocket::Ws,
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
        let wide_view = get_setting_wide_view();
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
            service_worker: service_worker,
            page_root: root.clone(),
            wide_view: wide_view,
            top: lunk::List::new(vec![]),
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
            bg_pushing: Default::default(),
            bg_pulling_interval: Default::default(),
            bg_pulling: Default::default(),
        })));
        let cs = get_stored_api_channels(None);
        let cgs = get_stored_api_channelgroups(None);
        merge_top(pc, cs, cgs);

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

        fn handle_notification(eg: &EventGraph, channel: &QualifiedChannelId, offset: ActivityOffset) {
            let state = state();
            let current_chat = state.current_chat.borrow();
            let Some(c) = &*current_chat else {
                return;
            };
            let cf = c.chat_state2.channel_lookup.borrow();
            if let Some(f) = cf.get(channel) {
                f.channel.notify(eg, offset);
            }
        }

        EventListener::new(&window().navigator().service_worker(), "message", {
            let eg = pc.eg();
            move |ev| {
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
                        handle_notification(&eg, &n.channel, n.offset);
                    },
                }
                ev.data();
            }
        }).forget();
        schedule_trigger_pull(pc.eg());
        let ws: Rc<RefCell<Option<Ws<(), s2c::Notification>>>> = Rc::new(RefCell::new(None));
        let create_ws = {
            let ws = ws.clone();
            let eg = pc.eg();
            move || {
                *ws.borrow_mut() = Some(Ws::new(state().log.clone(), &state().env.base_url, "", {
                    let eg = eg.clone();
                    move |_ws, m: s2c::Notification| {
                        handle_notification(&eg, &m.channel, m.offset);
                    }
                }));
            }
        };
        create_ws();
        EventListener::new(&window(), "focus", {
            let eg = pc.eg();
            move |_| {
                schedule_trigger_pull(eg.clone());
                create_ws();
            }
        }).forget();
        EventListener::new(&window(), "blur", {
            move |_| {
                *state().bg_pulling_interval.borrow_mut() = None;
                *ws.borrow_mut() = None;
            }
        }).forget();
        trigger_push();
        root.ref_own(|_| (
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
        ));

        // Root and display
        if wide_view {
            set_root(vec![style_export::cont_root_wide(style_export::ContRootWideArgs {
                menu: page_top::build(pc),
                page: root,
            }).root]);
        } else {
            set_root(vec![root]);
        }
    });
}
