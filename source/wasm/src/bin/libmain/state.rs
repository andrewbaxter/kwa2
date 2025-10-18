use {
    crate::libmain::{
        page_identities::build_page_identities,
        page_top::build_page_top,
    },
    gloo::{
        storage::{
            LocalStorage,
            Storage,
        },
        utils::{
            document,
            window,
        },
    },
    js_sys::decode_uri,
    lunk::{
        EventGraph,
        Prim,
        ProcessingContext,
    },
    rooting::El,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::wire::shared::{
        ChannelId,
        InternalChannelGroupId,
        InternalChannelId,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        collections::HashMap,
        rc::Rc,
    },
    wasm::{
        async_::BgVal,
        js::{
            get_dom_octothorpe,
            style_export,
            Env,
            Log,
            LogJsErr,
            VecLog,
        },
    },
    wasm_bindgen::JsValue,
};

pub const LOCALSTORAGE_PWA_MINISTATE: &str = "pwa_ministate";
pub const SESSIONSTORAGE_POST_REDIRECT_MINISTATE: &str = "post_redirect";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateChannel {
    pub identity: Identity,
    pub channel: ChannelId,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Ministate {
    Top,
    Identities,
    Identity(Identity),
    Channel(MinistateChannel),
    ChannelGroup(InternalChannelGroupId),
}

pub fn ministate_octothorpe(s: &Ministate) -> String {
    return format!("#{}", serde_json::to_string(&s).unwrap());
}

/// Replaces current state in history, no page change
pub fn record_replace_ministate(log: &Rc<dyn Log>, s: &Ministate) {
    window()
        .history()
        .unwrap()
        .replace_state_with_url(&JsValue::null(), "", Some(&ministate_octothorpe(s)))
        .unwrap();
    LocalStorage::set(LOCALSTORAGE_PWA_MINISTATE, s).log(log, "Error storing PWA ministate");
}

pub fn read_ministate(log: &Rc<dyn Log>) -> Ministate {
    let Some(s) = get_dom_octothorpe(log) else {
        return Ministate::Top;
    };
    match serde_json::from_str::<Ministate>(s.as_ref()) {
        Ok(s) => return s,
        Err(e) => {
            log.log(&format!("Unable to parse url anchor state (1/2, no urldecode) [{}]: {}", s, e));
        },
    };
    match serde_json::from_str::<Ministate>(&decode_uri(s.as_str()).unwrap().as_string().unwrap()) {
        Ok(s) => return s,
        Err(e) => {
            log.log(&format!("Unable to parse url anchor state (2/2, urldecode) [{}]: {}", s, e));
        },
    }
    return Ministate::Top;
}

pub struct State_ {
    pub eg: EventGraph,
    pub root: El,
    pub ministate: RefCell<Ministate>,
    pub env: Env,
    pub log: Rc<dyn Log>,
    pub log1: Rc<VecLog>,
}

thread_local!{
    pub(crate) static STATE: RefCell<Option<Rc<State_>>> = RefCell::new(None);
}

pub fn state() -> Rc<State_> {
    return STATE.with(|x| x.borrow().clone()).unwrap();
}

fn set_page(pc: &mut ProcessingContext, body: El) {
    let r = &state().root;
    r.ref_clear();
    r.ref_push(body);
}

pub fn build_ministate(pc: &mut ProcessingContext, s: &Ministate) {
    match s {
        Ministate::Top => {
            let body = build_page_top(pc);
            set_page(pc, body);
        },
        Ministate::Identities => {
            let body = build_page_identities(pc);
            set_page(pc, body);
        },
        Ministate::Identity(id) => {
            let body = build_page_identity(pc, id);
            set_page(pc, body);
        },
        Ministate::Channel(id) => {
            //. set_page(pc, build_page_channel(id));
        },
        Ministate::ChannelGroup(id) => {
            //. set_page(pc, build_page_channel_group(id));
        },
    }
}
