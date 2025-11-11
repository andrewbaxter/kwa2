use {
    crate::{
        async_::BgVal,
        chat_entry::ChatEntry,
        infinite::Infinite,
        js::{
            get_dom_octothorpe,
            Env,
            Log,
            LogJsErr,
            VecLog,
        },
        page_channel,
        page_channel_delete,
        page_channel_edit,
        page_channel_join_url,
        page_channel_menu,
        page_channel_new,
        page_channelgroup,
        page_channelgroup_delete,
        page_channelgroup_edit,
        page_channelgroup_menu,
        page_channelgroup_new,
        page_channelinvite,
        page_channelinvite_delete,
        page_channelinvite_edit,
        page_channelinvite_new,
        page_channelinvites,
        page_identities,
        page_identity,
        page_identity_delete,
        page_identity_edit,
        page_identity_new,
        page_identityinvite,
        page_identityinvite_delete,
        page_identityinvite_edit,
        page_identityinvite_new,
        page_identityinvites,
        page_settings,
        page_top,
        page_top_add,
    },
    futures::channel::oneshot,
    gloo::{
        storage::{
            LocalStorage,
            Storage,
        },
        utils::window,
    },
    js_sys::decode_uri,
    lunk::{
        EventGraph,
        ProcessingContext,
    },
    rooting::{
        spawn_rooted,
        El,
    },
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::shared::{
        ChannelGroupId,
        ChannelInviteId,
        IdentityInviteId,
        QualifiedChannelId,
        QualifiedMessageId,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        future::Future,
        rc::Rc,
    },
    wasm_bindgen::JsValue,
    wasm_bindgen_futures::spawn_local,
    web_sys::ServiceWorkerRegistration,
};

pub const LOCALSTORAGE_PWA_MINISTATE: &str = "pwa_ministate";
pub const SESSIONSTORAGE_POST_REDIRECT_MINISTATE: &str = "post_redirect";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateChannel {
    pub channel: QualifiedChannelId,
    pub reset: Option<QualifiedMessageId>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateChannelInvite {
    pub channel: QualifiedChannelId,
    pub reset: Option<QualifiedMessageId>,
    pub invite: ChannelInviteId,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateChannelGroup {
    pub channelgroup: ChannelGroupId,
    pub reset: Option<QualifiedMessageId>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MinistateIdentityInvite {
    pub identity: Identity,
    pub invite: IdentityInviteId,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Ministate {
    Top,
    Settings,
    TopAdd,
    ChannelJoinUrl,
    ChannelNew,
    ChannelGroupNew,
    Identities,
    IdentitiesNew,
    Identity(Identity),
    IdentityEdit(Identity),
    IdentityDelete(Identity),
    IdentityInvites(Identity),
    IdentityInviteNew(Identity),
    IdentityInvite(MinistateIdentityInvite),
    IdentityInviteEdit(MinistateIdentityInvite),
    IdentityInviteDelete(MinistateIdentityInvite),
    Channel(MinistateChannel),
    ChannelMenu(MinistateChannel),
    ChannelEdit(MinistateChannel),
    ChannelDelete(MinistateChannel),
    ChannelInvites(MinistateChannel),
    ChannelInviteNew(MinistateChannel),
    ChannelInvite(MinistateChannelInvite),
    ChannelInviteEdit(MinistateChannelInvite),
    ChannelInviteDelete(MinistateChannelInvite),
    ChannelGroup(MinistateChannelGroup),
    ChannelGroupMenu(MinistateChannelGroup),
    ChannelGroupEdit(MinistateChannelGroup),
    ChannelGroupDelete(MinistateChannelGroup),
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
        .log(log, &"Error replacing last history entry");
    LocalStorage::set(LOCALSTORAGE_PWA_MINISTATE, s).log(log, &"Error storing PWA ministate");
}

pub fn goto_replace_ministate(pc: &mut ProcessingContext, log: &Rc<dyn Log>, s: &Ministate) {
    window()
        .history()
        .unwrap()
        .push_state_with_url(&JsValue::null(), "", Some(&ministate_octothorpe(s)))
        .log(log, &"Error pushing history");
    LocalStorage::set(LOCALSTORAGE_PWA_MINISTATE, s).log(log, &"Error storing PWA ministate");
    build_ministate(pc, s);
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
    pub service_worker: BgVal<Result<ServiceWorkerRegistration, String>>,
    pub root: El,
    pub ministate: RefCell<Ministate>,
    pub env: Env,
    pub log: Rc<dyn Log>,
    pub log1: Rc<VecLog>,
    pub current_chat: RefCell<Option<Infinite<ChatEntry>>>,
}

thread_local!{
    pub static STATE: RefCell<Option<Rc<State_>>> = RefCell::new(None);
}

pub fn state() -> Rc<State_> {
    return STATE.with(|x| x.borrow().clone()).unwrap();
}

fn set_page(body: El) {
    let r = &state().root;
    r.ref_clear();
    r.ref_push(body);
}

pub fn build_ministate(pc: &mut ProcessingContext, s: &Ministate) {
    let body;
    match s {
        Ministate::Top => {
            body = page_top::build(pc);
        },
        Ministate::Settings => {
            body = page_settings::build();
        },
        Ministate::TopAdd => {
            body = page_top_add::build();
        },
        Ministate::Identities => {
            body = page_identities::build(pc);
        },
        Ministate::IdentitiesNew => {
            body = page_identity_new::build(pc);
        },
        Ministate::Identity(id) => {
            body = page_identity::build(id);
        },
        Ministate::IdentityEdit(id) => {
            body = page_identity_edit::build(pc, id);
        },
        Ministate::IdentityDelete(id) => {
            body = page_identity_delete::build(pc, id);
        },
        Ministate::IdentityInvites(id) => {
            body = page_identityinvites::build(pc, id);
        },
        Ministate::IdentityInviteNew(id) => {
            body = page_identityinvite_new::build(pc, id);
        },
        Ministate::IdentityInvite(s) => {
            body = page_identityinvite::build(&s.identity, &s.invite);
        },
        Ministate::IdentityInviteEdit(s) => {
            body = page_identityinvite_edit::build(pc, &s.identity, &s.invite);
        },
        Ministate::IdentityInviteDelete(s) => {
            body = page_identityinvite_delete::build(pc, &s.identity, &s.invite);
        },
        Ministate::ChannelNew => {
            body = page_channel_new::build(pc);
        },
        Ministate::ChannelJoinUrl => {
            body = page_channel_join_url::build(pc);
        },
        Ministate::Channel(s) => {
            body = page_channel::build(pc, &s.channel, &s.reset);
        },
        Ministate::ChannelMenu(s) => {
            body = page_channel_menu::build(&s.channel, &s.reset);
        },
        Ministate::ChannelEdit(s) => {
            body = page_channel_edit::build(pc, &s.channel, &s.reset);
        },
        Ministate::ChannelDelete(s) => {
            body = page_channel_delete::build(pc, &s.channel, &s.reset);
        },
        Ministate::ChannelInvites(s) => {
            body = page_channelinvites::build(&s.channel, &s.reset);
        },
        Ministate::ChannelInviteNew(s) => {
            body = page_channelinvite_new::build(pc, &s.channel, &s.reset);
        },
        Ministate::ChannelInvite(s) => {
            body = page_channelinvite::build(&s.channel, &s.invite, &s.reset);
        },
        Ministate::ChannelInviteEdit(s) => {
            body = page_channelinvite_edit::build(pc, &s.channel, &s.invite, &s.reset);
        },
        Ministate::ChannelInviteDelete(s) => {
            body = page_channelinvite_delete::build(pc, &s.channel, &s.invite, &s.reset);
        },
        Ministate::ChannelGroupNew => {
            body = page_channelgroup_new::build(pc);
        },
        Ministate::ChannelGroup(s) => {
            body = page_channelgroup::build(pc, &s.channelgroup, &s.reset);
        },
        Ministate::ChannelGroupMenu(s) => {
            body = page_channelgroup_menu::build(&s.channelgroup, &s.reset);
        },
        Ministate::ChannelGroupEdit(s) => {
            body = page_channelgroup_edit::build(pc, &s.channelgroup, &s.reset);
        },
        Ministate::ChannelGroupDelete(s) => {
            body = page_channelgroup_delete::build(pc, &s.channelgroup, &s.reset);
        },
    }
    set_page(body);
}

pub fn spawn_rooted_log(
    message: &'static str,
    f: impl Future<Output = Result<(), String>> + 'static,
) -> oneshot::Receiver<()> {
    return spawn_rooted(async move {
        if let Err(e) = f.await {
            state().log.log(&format!("Error in background task [{}]: {}", message, e));
        }
    });
}

pub fn spawn_log(message: &'static str, f: impl Future<Output = Result<(), String>> + 'static) {
    spawn_local(async move {
        if let Err(e) = f.await {
            state().log.log(&format!("Error in background task [{}]: {}", message, e));
        }
    });
}
