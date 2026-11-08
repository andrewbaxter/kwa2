use {
    crate::interface::shared::{
        ChannelGroupId,
        ChannelId,
        ChannelInviteId,
        IdentityInviteId,
        Message,
        MessageBody,
        MessageClientId,
        MessageId,
        QualifiedChannelId,
        QualifiedChannelInviteToken,
        QualifiedIdentityInviteToken,
        QualifiedMessageId,
    },
    glove::reqresp,
    jiff::Timestamp,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
        de::DeserializeOwned,
    },
    spaghettinuum::interface::identity::Identity,
    std::{
        borrow::Cow,
        str::FromStr,
    },
};

// POST/JSON data
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ModifyOption<T> {
    pub value: Option<T>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Logout;

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NotificationRegister {
    /// Matches `toJson()` result of push subscription object in js.
    pub data: serde_json::Value,
}

// # Identity
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityCreate {
    pub idem: Option<String>,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub shown_name: Option<String>,
    #[serde(default)]
    pub shown_description: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityRes {
    pub id: Identity,
    pub idem: Option<String>,
    pub name: String,
    pub description: String,
    pub shown_name: Option<String>,
    pub shown_description: Option<String>,
    pub portrait: Option<crate::interface::shared::FileRef>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityModify {
    pub id: Identity,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub shown_name: Option<ModifyOption<String>>,
    #[serde(default)]
    pub shown_description: Option<ModifyOption<String>>,
    #[serde(default)]
    pub portrait: Option<ModifyOption<crate::interface::shared::Portrait>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityDelete {
    pub id: Identity,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityList;

// # Identity invitation
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInviteCreate {
    #[serde(default)]
    pub idem: Option<String>,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub shown_name: Option<String>,
    #[serde(default)]
    pub shown_description: Option<String>,
    pub identity: Identity,
    #[serde(default)]
    pub single_use: bool,
    #[serde(default)]
    pub expiry: Option<Timestamp>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInviteRes {
    pub id: IdentityInviteId,
    pub token: QualifiedIdentityInviteToken,
    pub name: String,
    pub description: String,
    pub shown_name: Option<String>,
    pub shown_description: Option<String>,
    pub single_use: bool,
    pub expiry: Option<Timestamp>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInviteModify {
    pub id: IdentityInviteId,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub shown_name: Option<ModifyOption<String>>,
    #[serde(default)]
    pub shown_description: Option<ModifyOption<String>>,
    #[serde(default)]
    pub single_use: Option<bool>,
    #[serde(default)]
    pub expiry: Option<ModifyOption<Timestamp>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInviteDelete {
    pub id: IdentityInviteId,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInviteList;

// # Channel group
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupCreate {
    pub idem: Option<String>,
    pub name: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupRes {
    pub id: ChannelGroupId,
    pub idem: Option<String>,
    pub name: String,
    pub description: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupList;

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupModify {
    pub id: ChannelGroupId,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupDelete {
    pub id: ChannelGroupId,
}

// # Channel
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelCreate {
    pub identity: Identity,
    pub idem: Option<String>,
    pub group: Option<ChannelGroupId>,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub shown_name: Option<String>,
    #[serde(default)]
    pub shown_description: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ChannelRetention {
    Nothing,
    Viewed,
    All,
    Days(u64),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelRes {
    pub own_identity: Identity,
    pub id: QualifiedChannelId,
    pub idem: Option<String>,
    pub group: Option<ChannelGroupId>,
    pub name: String,
    pub description: String,
    pub shown_name: Option<String>,
    pub shown_description: Option<String>,
    pub portrait: Option<crate::interface::shared::FileRef>,
    pub retention: ChannelRetention,
    pub public_read: bool,
    pub public_post: bool,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelList;

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelModify {
    pub own_identity: Identity,
    pub id: QualifiedChannelId,
    #[serde(default)]
    pub group: Option<ModifyOption<ChannelGroupId>>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub shown_name: Option<ModifyOption<String>>,
    #[serde(default)]
    pub shown_description: Option<ModifyOption<String>>,
    #[serde(default)]
    pub retention: Option<ChannelRetention>,
    #[serde(default)]
    pub public_read: Option<bool>,
    #[serde(default)]
    pub public_post: Option<bool>,
    #[serde(default)]
    pub portrait: Option<ModifyOption<crate::interface::shared::Portrait>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelDelete {
    pub id: QualifiedChannelId,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelJoinChannel {
    pub channel: QualifiedChannelId,
    pub sender: Identity,
    pub code: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub portrait: Option<crate::interface::shared::JoinPortrait>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelJoinIdentity {
    pub identity: Identity,
    pub sender: Identity,
    pub code: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub portrait: Option<crate::interface::shared::JoinPortrait>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelJoinPreview {
    pub url: crate::interface::wire::kwaurl::KwaUrl,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelJoinPreviewRes {
    pub name: String,
    pub description: String,
    pub portrait: Option<crate::interface::shared::FileRef>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelAdd {
    pub channel: QualifiedChannelId,
    pub identity: Identity,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub portrait: Option<crate::interface::shared::JoinPortrait>,
}

// # Channel member
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelMemberList {
    pub channel: QualifiedChannelId,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelMemberRes {
    pub identity: Identity,
    pub can_post: bool,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelMemberModify {
    pub channel: QualifiedChannelId,
    pub member: Identity,
    #[serde(default)]
    pub can_post: Option<bool>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelMemberDelete {
    pub channel: QualifiedChannelId,
    pub member: Identity,
}

// # Channel invitation
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteCreate {
    pub channel: QualifiedChannelId,
    #[serde(default)]
    pub idem: Option<String>,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub shown_name: Option<String>,
    #[serde(default)]
    pub shown_description: Option<String>,
    #[serde(default)]
    pub single_use: bool,
    #[serde(default)]
    pub expiry: Option<Timestamp>,
    #[serde(default = "default_true")]
    pub can_post: bool,
}

fn default_true() -> bool {
    return true;
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteRes {
    pub id: ChannelInviteId,
    pub token: QualifiedChannelInviteToken,
    pub name: String,
    pub description: String,
    pub shown_name: Option<String>,
    pub shown_description: Option<String>,
    pub single_use: bool,
    pub expiry: Option<Timestamp>,
    pub can_post: bool,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteModify {
    pub id: ChannelInviteId,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub shown_name: Option<ModifyOption<String>>,
    #[serde(default)]
    pub shown_description: Option<ModifyOption<String>>,
    #[serde(default)]
    pub single_use: Option<bool>,
    #[serde(default)]
    pub expiry: Option<ModifyOption<Timestamp>>,
    #[serde(default)]
    pub can_post: Option<bool>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteDelete {
    pub id: ChannelInviteId,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteList;

// Contacts
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ContactRes {
    pub id: Identity,
    pub name: String,
    pub description: String,
    pub portrait: Option<crate::interface::shared::FileRef>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ContactList;

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ContactModify {
    pub id: Identity,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub portrait: Option<ModifyOption<crate::interface::shared::Portrait>>,
}

// Message
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessagePush {
    pub client_id: MessageClientId,
    pub channel: QualifiedChannelId,
    pub identity: Identity,
    #[serde(default = "message_rel_default")]
    pub rel: crate::interface::shared::MessageRel,
    pub body: String,
}

fn message_rel_default() -> crate::interface::shared::MessageRel {
    return crate::interface::shared::MessageRel::None;
}

reqresp!(pub proto {
    Logout(Logout) =>(),
    NotificationRegister(NotificationRegister) =>(),
    IdentityCreate(IdentityCreate) => IdentityRes,
    IdentityModify(IdentityModify) => IdentityRes,
    IdentityDelete(IdentityDelete) =>(),
    IdentityList(IdentityList) => Vec < IdentityRes >,
    IdentityInviteCreate(IdentityInviteCreate) => IdentityInviteRes,
    IdentityInviteModify(IdentityInviteModify) => IdentityInviteRes,
    IdentityInviteDelete(IdentityInviteDelete) =>(),
    IdentityInviteList(IdentityInviteList) => Vec < IdentityInviteRes >,
    ChannelGroupCreate(ChannelGroupCreate) => ChannelGroupRes,
    ChannelGroupModify(ChannelGroupModify) => ChannelGroupRes,
    ChannelGroupDelete(ChannelGroupDelete) =>(),
    ChannelGroupList(ChannelGroupList) => Vec < ChannelGroupRes >,
    ChannelCreate(ChannelCreate) => ChannelRes,
    ChannelJoinChannel(ChannelJoinChannel) => ChannelRes,
    ChannelJoinIdentity(ChannelJoinIdentity) => ChannelRes,
    ChannelAdd(ChannelAdd) => ChannelRes,
    ChannelJoinPreview(ChannelJoinPreview) => ChannelJoinPreviewRes,
    ChannelModify(ChannelModify) => ChannelRes,
    ChannelDelete(ChannelDelete) =>(),
    ChannelList(ChannelList) => Vec < ChannelRes >,
    ChannelMemberList(ChannelMemberList) => Vec < ChannelMemberRes >,
    ChannelMemberModify(ChannelMemberModify) =>(),
    ChannelMemberDelete(ChannelMemberDelete) =>(),
    ChannelInviteCreate(ChannelInviteCreate) => ChannelInviteRes,
    ChannelInviteModify(ChannelInviteModify) => ChannelInviteRes,
    ChannelInviteDelete(ChannelInviteDelete) =>(),
    ChannelInviteList(ChannelInviteList) => Vec < ChannelInviteRes >,
    ContactList(ContactList) => Vec < ContactRes >,
    ContactModify(ContactModify) => ContactRes,
    MessagePush(MessagePush) =>(),
});

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SnapOffset(pub usize);

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SnapOffsetPos {
    pub offset: SnapOffset,
    pub pos: PagePosition,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SnapPage(pub usize);

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PagePosition {
    First,
    Middle,
    Last,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SnapPageOffsetPos {
    pub page: SnapPage,
    pub offset_pos: SnapOffsetPos,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ActivityOffset(pub usize);

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ActivityOffsetPos {
    pub offset: ActivityOffset,
    pub pos: PagePosition,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ActivityPage(pub usize);

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ActivityPageOffsetPos {
    pub page: ActivityPage,
    pub offset_pos: ActivityOffsetPos,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SnapMessage {
    pub offset_pos: SnapOffsetPos,
    pub original_id: QualifiedMessageId,
    pub original_receive_time: Timestamp,
    pub client_id: Option<MessageClientId>,
    pub message: MessageBody,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SnapPageRes {
    pub latest_activity: Option<ActivityPageOffsetPos>,
    pub messages: Vec<SnapMessage>,
}

// # GET/Path data
//
// ---
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GetResp<T> {
    Ok(T),
    OwnerUnreachable,
}

pub trait PathReqTrait: Sized {
    type Resp: DeserializeOwned;

    fn deserialize_path(path: &str) -> Result<Self, String>;
    fn serialize_path(&self) -> String;
}

pub fn serialize_path(x: impl IntoIterator<Item = String>) -> String {
    return x.into_iter().map(|x| urlencoding::encode(&x).into_owned()).collect::<Vec<_>>().join("/");
}

pub fn deserialize_path<'a>(x: &'a str) -> impl Iterator<Item = Cow<'a, str>> {
    return x.split("/").map(|x| urlencoding::decode(x).unwrap_or(std::borrow::Cow::Borrowed(x)));
}

pub fn confirm_path_const<'a>(segs: &mut dyn Iterator<Item = Cow<'a, str>>, want: &str) -> Result<(), String> {
    let next = segs.next();
    let Some(x) = next.as_ref().map(|x| x.as_ref()) else {
        return Err(format!("Path missing segment [{}]", want));
    };
    if x != want {
        return Err(format!("Path missing segment [{}]", want));
    }
    return Ok(());
}

pub fn confirm_path_empty<'a>(segs: &mut dyn Iterator<Item = Cow<'a, str>>) -> Result<(), String> {
    if segs.next().is_some() {
        return Err(format!("Unrecognized extra path segments"));
    }
    return Ok(());
}

pub fn confirm_path_element<
    'a,
    E: std::fmt::Display,
    T: FromStr<Err = E>,
>(segs: &mut dyn Iterator<Item = Cow<'a, str>>, hint: &str) -> Result<T, String> {
    let Some(v) = segs.next() else {
        return Err(format!("Path missing segment, {}", hint));
    };
    return Ok(T::from_str(&v).map_err(|e| format!("Couldn't parse path segment, {}: {}", hint, e))?);
}
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NotificationServerKey;

const PATH_PREFIX_NOTIFICATION_SERVER_KEY: &str = "notification_server_key";

impl PathReqTrait for NotificationServerKey {
    // b64-encoded key as required by js
    type Resp = String;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_NOTIFICATION_SERVER_KEY)?;
        confirm_path_empty(&mut parts)?;
        return Ok(NotificationServerKey);
    }

    fn serialize_path(&self) -> String {
        return serialize_path([PATH_PREFIX_NOTIFICATION_SERVER_KEY.to_string()]);
    }
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SnapByRes {
    pub original_receive_time: Timestamp,
    pub offset: SnapPageOffsetPos,
}

pub struct SnapById {
    pub id: QualifiedMessageId,
}

const PATH_PREFIX_SNAP_BY_ID: &str = "snap_by_id";

impl PathReqTrait for SnapById {
    type Resp = Option<SnapByRes>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_SNAP_BY_ID)?;
        let out = Self { id: QualifiedMessageId {
            channel: QualifiedChannelId {
                identity: confirm_path_element(&mut parts, "channel identity")?,
                channel: ChannelId(confirm_path_element(&mut parts, "channel")?),
            },
            message: MessageId {
                identity: confirm_path_element(&mut parts, "message identity")?,
                unique: confirm_path_element(&mut parts, "message unique")?,
            },
        } };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [
                PATH_PREFIX_SNAP_BY_ID.to_string(),
                self.id.channel.identity.to_string(),
                self.id.channel.channel.0.to_string(),
                self.id.message.identity.to_string(),
                self.id.message.unique.to_string(),
            ],
        );
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageRevision {
    pub receive_time: Timestamp,
    pub body: String,
}

pub struct MessageHistory {
    pub id: QualifiedMessageId,
}

const PATH_PREFIX_MESSAGE_HISTORY: &str = "message_history";

impl PathReqTrait for MessageHistory {
    type Resp = Option<Vec<MessageRevision>>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_MESSAGE_HISTORY)?;
        let out = Self { id: QualifiedMessageId {
            channel: QualifiedChannelId {
                identity: confirm_path_element(&mut parts, "channel identity")?,
                channel: ChannelId(confirm_path_element(&mut parts, "channel")?),
            },
            message: MessageId {
                identity: confirm_path_element(&mut parts, "message identity")?,
                unique: confirm_path_element(&mut parts, "message unique")?,
            },
        } };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [
                PATH_PREFIX_MESSAGE_HISTORY.to_string(),
                self.id.channel.identity.to_string(),
                self.id.channel.channel.0.to_string(),
                self.id.message.identity.to_string(),
                self.id.message.unique.to_string(),
            ],
        );
    }
}

pub struct SnapByClientId {
    pub channel: QualifiedChannelId,
    pub client_id: MessageClientId,
}

const PATH_PREFIX_SNAP_BY_CLIENT_ID: &str = "snap_by_client_id";

impl PathReqTrait for SnapByClientId {
    type Resp = Option<SnapByRes>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_SNAP_BY_CLIENT_ID)?;
        let out = SnapByClientId {
            channel: QualifiedChannelId {
                identity: confirm_path_element(&mut parts, "channel identity")?,
                channel: ChannelId(confirm_path_element(&mut parts, "channel")?),
            },
            client_id: MessageClientId(confirm_path_element(&mut parts, "message client id")?),
        };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [
                PATH_PREFIX_SNAP_BY_CLIENT_ID.to_string(),
                self.channel.identity.to_string(),
                self.channel.channel.0.to_string(),
                self.client_id.0.to_string(),
            ],
        );
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct OutboxEntry {
    pub sender: Identity,
    pub client_id: MessageClientId,
    pub created: Timestamp,
    pub id: MessageId,
    pub rel: crate::interface::shared::MessageRel,
    pub body: String,
}

pub struct OutboxList {
    pub channel: QualifiedChannelId,
}

const PATH_PREFIX_OUTBOX: &str = "outbox";

impl PathReqTrait for OutboxList {
    type Resp = Vec<OutboxEntry>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_OUTBOX)?;
        let out = OutboxList { channel: QualifiedChannelId {
            identity: confirm_path_element(&mut parts, "channel identity")?,
            channel: ChannelId(confirm_path_element(&mut parts, "channel")?),
        } };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [PATH_PREFIX_OUTBOX.to_string(), self.channel.identity.to_string(), self.channel.channel.0.to_string()],
        );
    }
}

pub struct MessageIdByClientId {
    pub channel: QualifiedChannelId,
    pub client_id: MessageClientId,
}

const PATH_PREFIX_MESSAGE_ID_BY_CLIENT_ID: &str = "message_id_by_client_id";

impl PathReqTrait for MessageIdByClientId {
    type Resp = Option<MessageId>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_MESSAGE_ID_BY_CLIENT_ID)?;
        let out = MessageIdByClientId {
            channel: QualifiedChannelId {
                identity: confirm_path_element(&mut parts, "channel identity")?,
                channel: ChannelId(confirm_path_element(&mut parts, "channel")?),
            },
            client_id: MessageClientId(confirm_path_element(&mut parts, "message client id")?),
        };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [
                PATH_PREFIX_MESSAGE_ID_BY_CLIENT_ID.to_string(),
                self.channel.identity.to_string(),
                self.channel.channel.0.to_string(),
                self.client_id.0.to_string(),
            ],
        );
    }
}

pub struct SnapPageContainingTime {
    pub channel: QualifiedChannelId,
    pub time: Timestamp,
}

const PATH_PREFIX_SNAP_PAGE_CONTAINING_TIME: &str = "snap_page_containing_time";

impl PathReqTrait for SnapPageContainingTime {
    type Resp = Option<SnapPage>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_SNAP_PAGE_CONTAINING_TIME)?;
        let out = SnapPageContainingTime {
            channel: QualifiedChannelId {
                identity: confirm_path_element(&mut parts, "channel identity")?,
                channel: ChannelId(confirm_path_element(&mut parts, "channel")?),
            },
            time: confirm_path_element(&mut parts, "message identity")?,
        };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [
                PATH_PREFIX_SNAP_PAGE_CONTAINING_TIME.to_string(),
                self.channel.identity.to_string(),
                self.channel.channel.0.to_string(),
                self.time.to_string(),
            ],
        );
    }
}

pub struct GetSnapPage {
    pub channel: QualifiedChannelId,
    pub page: SnapPage,
}

const PATH_PREFIX_SNAP_PAGE: &str = "snap_page";

impl PathReqTrait for GetSnapPage {
    type Resp = Option<SnapPageRes>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_SNAP_PAGE)?;
        let out = GetSnapPage {
            channel: QualifiedChannelId {
                identity: confirm_path_element(&mut parts, "channel identity")?,
                channel: ChannelId(confirm_path_element(&mut parts, "channel")?),
            },
            page: SnapPage(confirm_path_element(&mut parts, "snap page")?),
        };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [
                PATH_PREFIX_SNAP_PAGE.to_string(),
                self.channel.identity.to_string(),
                self.channel.channel.0.to_string(),
                self.page.0.to_string(),
            ],
        );
    }
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ActivityLatestAll;

const PATH_PREFIX_ACTIVITY_LATEST_ALL: &str = "activity_latest_all";

impl PathReqTrait for ActivityLatestAll {
    type Resp = Vec<(QualifiedChannelId, ActivityOffset)>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_ACTIVITY_LATEST_ALL)?;
        confirm_path_empty(&mut parts)?;
        return Ok(ActivityLatestAll);
    }

    fn serialize_path(&self) -> String {
        return serialize_path([PATH_PREFIX_ACTIVITY_LATEST_ALL.to_string()]);
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ActivityPageMessage {
    pub message: Message,
    pub offset_pos: ActivityOffsetPos,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ActivityPageRes {
    pub messages: Vec<ActivityPageMessage>,
}

pub struct GetActivityPage {
    pub channel: QualifiedChannelId,
    pub page: ActivityPage,
}

const PATH_PREFIX_ACTIVITY_PAGE: &str = "activity_page";

impl PathReqTrait for GetActivityPage {
    type Resp = Option<ActivityPageRes>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_ACTIVITY_PAGE)?;
        let out = GetActivityPage {
            channel: QualifiedChannelId {
                identity: confirm_path_element(&mut parts, "channel identity")?,
                channel: ChannelId(confirm_path_element(&mut parts, "channel")?),
            },
            page: ActivityPage(confirm_path_element(&mut parts, "page")?),
        };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [
                PATH_PREFIX_ACTIVITY_PAGE.to_string(),
                self.channel.identity.to_string(),
                self.channel.channel.0.to_string(),
                self.page.0.to_string(),
            ],
        );
    }
}

pub struct GetFile {
    pub hash: crate::interface::shared::FileHash,
}

const PATH_PREFIX_FILE: &str = "file";

impl PathReqTrait for GetFile {
    type Resp = Vec<u8>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_FILE)?;
        let out = GetFile { hash: confirm_path_element(&mut parts, "hash")? };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path([PATH_PREFIX_FILE.to_string(), self.hash.to_string()]);
    }
}

pub struct GetJoinPreviewFile {
    pub url: crate::interface::wire::kwaurl::KwaUrl,
    pub hash: crate::interface::shared::FileHash,
}

const PATH_PREFIX_JOIN_PREVIEW_FILE: &str = "join_preview_file";

impl PathReqTrait for GetJoinPreviewFile {
    type Resp = Vec<u8>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_JOIN_PREVIEW_FILE)?;
        let url: String = confirm_path_element(&mut parts, "url")?;
        let out = GetJoinPreviewFile {
            url: crate::interface::wire::kwaurl::KwaUrl::from_string(&url)?,
            hash: confirm_path_element(&mut parts, "hash")?,
        };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [PATH_PREFIX_JOIN_PREVIEW_FILE.to_string(), self.url.to_string(), self.hash.to_string()],
        );
    }
}
