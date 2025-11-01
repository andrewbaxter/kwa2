use {
    crate::{
        api::req_post_json,
        state::state,
    },
    flowcontrol::shed,
    gloo::storage::{
        LocalStorage,
        Storage,
    },
    jiff::Timestamp,
    serde::{
        de::DeserializeOwned,
        Deserialize,
        Serialize,
    },
    shared::interface::wire::{
        c2s::{
            self,
            ChannelGroupRes,
            ChannelInviteRes,
            ChannelRes,
            IdentityInviteRes,
            IdentityRes,
        },
        shared::{
            ChannelGroupId,
            ChannelInviteId,
            IdentityInviteId,
            QualifiedChannelId,
        },
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        collections::HashMap,
        hash::Hash,
    },
    crate::js::LogJsErr,
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct LocalValue<T> {
    pub last_used: Timestamp,
    pub res: T,
}

fn get_stored_values<
    'de,
    V: Serialize + DeserializeOwned,
    K: PartialEq,
>(k: &'static str, access_id: fn(&V) -> K, touch: Option<&K>) -> Vec<LocalValue<V>> {
    let mut out = match LocalStorage::get::<Vec<LocalValue<V>>>(k) {
        Ok(identities) => identities,
        Err(e) => {
            match e {
                gloo::storage::errors::StorageError::SerdeError(e) => {
                    state().log.log(&format!("Error loading [{}] from local storage: {}", k, e));
                },
                gloo::storage::errors::StorageError::KeyNotFound(_) => {
                    // nop
                },
                gloo::storage::errors::StorageError::JsError(e) => {
                    state().log.log(&format!("Error loading [{}] from local storage: {}", k, e));
                },
            }
            Default::default()
        },
    };
    if let Some(touch) = touch {
        for v in &mut out {
            if access_id(&v.res) == *touch {
                v.last_used = Timestamp::now();
            }
        }
    }
    return out;
}

async fn req_api_values<
    'de,
    V: Serialize + DeserializeOwned,
    R: c2s::proto::ReqTrait<Resp = Vec<V>>,
    K: Eq + Hash,
>(k: &'static str, r: R, access_id: fn(&V) -> K, touch: Option<&K>) -> Result<Vec<LocalValue<V>>, String> {
    let old_vs =
        get_stored_values(k, access_id, None)
            .into_iter()
            .map(|v| (access_id(&v.res), v))
            .collect::<HashMap<_, _>>();
    let vs = req_post_json(&state().env.base_url, r).await?;
    let mut out = vec![];
    let now = Timestamp::now();
    for v in vs {
        out.push(LocalValue {
            last_used: shed!{
                if let Some(touch) = touch {
                    if *touch == access_id(&v) {
                        break now;
                    }
                }
                if let Some(old_v) = old_vs.get(&access_id(&v)) {
                    break old_v.last_used;
                }
                break now;
            },
            res: v,
        });
    }
    LocalStorage::set(k, &out).log(&state().log, &"Failed to set local storage");
    return Ok(out);
}

async fn ensure_api_value<
    'de,
    V: Serialize + DeserializeOwned,
    K: PartialEq,
>(k: &'static str, access_id: fn(&V) -> K, v: V) {
    let mut values = get_stored_values(k, access_id, None);
    let found = values.iter().enumerate().find_map(|(i, x)| if access_id(&x.res) == access_id(&v) {
        Some(i)
    } else {
        None
    });
    let new_value = LocalValue {
        last_used: Timestamp::now(),
        res: v,
    };
    if let Some(found) = found {
        values.splice(found ..= found, vec![new_value]);
    } else {
        values.push(new_value);
    }
    LocalStorage::set(k, values).log(&state().log, &format_args!("Failed to set local storage at key [{}]", k));
}

async fn delete_api_value<
    'de,
    V: Serialize + DeserializeOwned,
    K: PartialEq,
>(k: &'static str, access_id: fn(&V) -> K, v: V) {
    let values = get_stored_values(k, access_id, None);
    let values = values.iter().filter(|x| access_id(&x.res) != access_id(&v)).collect::<Vec<_>>();
    LocalStorage::set(k, values).log(&state().log, &format_args!("Failed to set local storage at key [{}]", k));
}

// Identity
const LOCALSTORAGE_IDENTITIES: &str = "identities";
pub type LocalIdentity = LocalValue<IdentityRes>;

pub fn get_stored_api_identities(touch: Option<&Identity>) -> Vec<LocalIdentity> {
    return get_stored_values(LOCALSTORAGE_IDENTITIES, |x| x.id.clone(), touch);
}

pub async fn req_api_identities(touch: Option<&Identity>) -> Result<Vec<LocalIdentity>, String> {
    return req_api_values(LOCALSTORAGE_IDENTITIES, c2s::IdentityList, |x| x.id, touch).await;
}

pub async fn ensure_identity(v: IdentityRes) {
    ensure_api_value(LOCALSTORAGE_IDENTITIES, |x| x.id, v).await;
}

pub async fn delete_identity(v: IdentityRes) {
    delete_api_value(LOCALSTORAGE_IDENTITIES, |x| x.id, v).await;
}

// Identity invites
const LOCALSTORAGE_IDENTITY_INVITES: &str = "identity_invites";
pub type LocalIdentityInvite = LocalValue<IdentityInviteRes>;

pub fn get_stored_api_identityinvites(touch: Option<&IdentityInviteId>) -> Vec<LocalIdentityInvite> {
    return get_stored_values(LOCALSTORAGE_IDENTITY_INVITES, |x| x.id, touch);
}

pub async fn req_api_identityinvites(touch: Option<&IdentityInviteId>) -> Result<Vec<LocalIdentityInvite>, String> {
    return req_api_values(LOCALSTORAGE_IDENTITY_INVITES, c2s::IdentityInviteList, |x| x.id, touch).await;
}

pub async fn ensure_identityinvite(v: IdentityInviteRes) {
    ensure_api_value(LOCALSTORAGE_IDENTITY_INVITES, |x| x.id, v).await;
}

pub async fn delete_identityinvite(v: IdentityInviteRes) {
    delete_api_value(LOCALSTORAGE_IDENTITY_INVITES, |x| x.id, v).await;
}

// Channelgroup
const LOCALSTORAGE_CHANNELGROUPS: &str = "channelgroups";
pub type LocalChannelGroup = LocalValue<ChannelGroupRes>;

pub fn get_stored_api_channelgroups(touch: Option<&ChannelGroupId>) -> Vec<LocalChannelGroup> {
    return get_stored_values(LOCALSTORAGE_CHANNELGROUPS, |x| x.id.clone(), touch);
}

pub async fn req_api_channelgroups(touch: Option<&ChannelGroupId>) -> Result<Vec<LocalChannelGroup>, String> {
    return req_api_values(LOCALSTORAGE_CHANNELGROUPS, c2s::ChannelGroupList, |x| x.id.clone(), touch).await;
}

pub async fn ensure_channelgroup(v: ChannelGroupRes) {
    ensure_api_value(LOCALSTORAGE_CHANNELGROUPS, |x| x.id, v).await;
}

pub async fn delete_channelgroup(v: ChannelGroupRes) {
    delete_api_value(LOCALSTORAGE_CHANNELGROUPS, |x| x.id, v).await;
}

// Channel
const LOCALSTORAGE_CHANNELS: &str = "channels";
pub type LocalChannel = LocalValue<ChannelRes>;

pub fn get_stored_api_channels(touch: Option<&QualifiedChannelId>) -> Vec<LocalChannel> {
    return get_stored_values(LOCALSTORAGE_IDENTITIES, |x| x.id.clone(), touch);
}

pub async fn req_api_channels(touch: Option<&QualifiedChannelId>) -> Result<Vec<LocalChannel>, String> {
    return req_api_values(LOCALSTORAGE_IDENTITIES, c2s::ChannelList, |x| x.id.clone(), touch).await;
}

pub async fn ensure_channel(v: ChannelRes) {
    ensure_api_value(LOCALSTORAGE_CHANNELS, |x| x.id.clone(), v).await;
}

pub async fn delete_channel(v: ChannelRes) {
    delete_api_value(LOCALSTORAGE_CHANNELS, |x| x.id.clone(), v).await;
}

// Identity invites
const LOCALSTORAGE_CHANNEL_INVITES: &str = "channel_invites";
pub type LocalChannelInvite = LocalValue<ChannelInviteRes>;

pub fn get_stored_api_channelinvites(touch: Option<&ChannelInviteId>) -> Vec<LocalChannelInvite> {
    return get_stored_values(LOCALSTORAGE_CHANNEL_INVITES, |x| x.id, touch);
}

pub async fn req_api_channelinvites(touch: Option<&ChannelInviteId>) -> Result<Vec<LocalChannelInvite>, String> {
    return req_api_values(LOCALSTORAGE_CHANNEL_INVITES, c2s::ChannelInviteList, |x| x.id, touch).await;
}

pub async fn ensure_channelinvite(v: ChannelInviteRes) {
    ensure_api_value(LOCALSTORAGE_CHANNEL_INVITES, |x| x.id, v).await;
}

pub async fn delete_channelinvite(v: ChannelInviteRes) {
    delete_api_value(LOCALSTORAGE_CHANNEL_INVITES, |x| x.id.clone(), v).await;
}
