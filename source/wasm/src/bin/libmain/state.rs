use {
    crate::libmain::{
        page_channel::{
            self,
        },
        page_channel_delete,
        page_channel_edit::{
            self,
        },
        page_channel_join_url::{
            self,
        },
        page_channel_menu::{
            self,
        },
        page_channel_new::{
            self,
        },
        page_channelgroup::{
            self,
        },
        page_channelgroup_delete,
        page_channelgroup_edit::{
            self,
        },
        page_channelgroup_menu::{
            self,
        },
        page_channelgroup_new::{
            self,
        },
        page_channelinvite::{
            self,
        },
        page_channelinvite_delete,
        page_channelinvite_edit::{
            self,
        },
        page_channelinvite_new::{
            self,
        },
        page_channelinvites::{
            self,
        },
        page_identities::{
            self,
        },
        page_identity::{
            self,
        },
        page_identity_delete,
        page_identity_edit::{
            self,
        },
        page_identity_new::{
            self,
        },
        page_identityinvite::{
            self,
        },
        page_identityinvite_delete,
        page_identityinvite_edit::{
            self,
        },
        page_identityinvite_new::{
            self,
        },
        page_identityinvites::{
            self,
        },
        page_top::{
            self,
        },
        page_top_add::{
            self,
        },
    },
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
    rooting::El,
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::wire::shared::{
        ChannelGroupId,
        ChannelInviteId,
        IdentityInviteId,
        QualifiedChannelId,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        cell::RefCell,
        rc::Rc,
    },
    wasm::js::{
        get_dom_octothorpe,
        Env,
        Log,
        LogJsErr,
        VecLog,
    },
    wasm_bindgen::JsValue,
};

pub const LOCALSTORAGE_PWA_MINISTATE: &str = "pwa_ministate";
pub const SESSIONSTORAGE_POST_REDIRECT_MINISTATE: &str = "post_redirect";

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Ministate {
    Top,
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
    IdentityInvite(IdentityInviteId),
    IdentityInviteEdit(IdentityInviteId),
    IdentityInviteDelete(IdentityInviteId),
    Channel(QualifiedChannelId),
    ChannelMenu(QualifiedChannelId),
    ChannelEdit(QualifiedChannelId),
    ChannelDelete(QualifiedChannelId),
    ChannelInvites(QualifiedChannelId),
    ChannelInviteNew(QualifiedChannelId),
    ChannelInvite(ChannelInviteId),
    ChannelInviteEdit(ChannelInviteId),
    ChannelInviteDelete(ChannelInviteId),
    ChannelGroup(ChannelGroupId),
    ChannelGroupMenu(ChannelGroupId),
    ChannelGroupEdit(ChannelGroupId),
    ChannelGroupDelete(ChannelGroupId),
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
        Ministate::IdentityInvite(id) => {
            body = page_identityinvite::build(id);
        },
        Ministate::IdentityInviteEdit(id) => {
            body = page_identityinvite_edit::build(pc, id);
        },
        Ministate::IdentityInviteDelete(id) => {
            body = page_identityinvite_delete::build(pc, id);
        },
        Ministate::ChannelNew => {
            body = page_channel_new::build(pc);
        },
        Ministate::ChannelJoinUrl => {
            body = page_channel_join_url::build(pc);
        },
        Ministate::Channel(id) => {
            body = page_channel::build(id);
        },
        Ministate::ChannelMenu(id) => {
            body = page_channel_menu::build(id);
        },
        Ministate::ChannelEdit(id) => {
            body = page_channel_edit::build(pc, id);
        },
        Ministate::ChannelDelete(id) => {
            body = page_channel_delete::build(pc, id);
        },
        Ministate::ChannelInvites(id) => {
            body = page_channelinvites::build(id);
        },
        Ministate::ChannelInviteNew(id) => {
            body = page_channelinvite_new::build(pc, id);
        },
        Ministate::ChannelInvite(id) => {
            body = page_channelinvite::build(id);
        },
        Ministate::ChannelInviteEdit(id) => {
            body = page_channelinvite_edit::build(pc, id);
        },
        Ministate::ChannelInviteDelete(id) => {
            body = page_channelinvite_delete::build(pc, id);
        },
        Ministate::ChannelGroupNew => {
            body = page_channelgroup_new::build(pc);
        },
        Ministate::ChannelGroup(id) => {
            body = page_channelgroup::build(id);
        },
        Ministate::ChannelGroupMenu(id) => {
            body = page_channelgroup_menu::build(id);
        },
        Ministate::ChannelGroupEdit(id) => {
            body = page_channelgroup_edit::build(pc, id);
        },
        Ministate::ChannelGroupDelete(id) => {
            body = page_channelgroup_delete::build(pc, id);
        },
    }
    set_page(body);
}
