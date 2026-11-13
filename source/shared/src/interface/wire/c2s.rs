use {
    crate::interface::shared::{
        ChannelGroupId,
        ChannelId,
        ChannelInviteId,
        IdentityInviteId,
        Level,
        Message,
        MessageBlock,
        MessageBody,
        MessageClientId,
        MessageId,
        QualifiedChannelId,
        QualifiedChannelInviteToken,
        QualifiedFileId,
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
    pub default_level: Option<Level>,
    pub level: Level,
    pub reminders: bool,
    pub reminder_lead_minutes: u64,
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
    pub default_level: Option<ModifyOption<Level>>,
    #[serde(default)]
    pub portrait: Option<ModifyOption<crate::interface::shared::Portrait>>,
    #[serde(default)]
    pub reminders: Option<bool>,
    #[serde(default)]
    pub reminder_lead_minutes: Option<u64>,
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
    pub level: Level,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelMemberModify {
    pub channel: QualifiedChannelId,
    pub member: Identity,
    #[serde(default)]
    pub level: Option<Level>,
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
    pub level: Level,
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
    pub level: Level,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteModify {
    pub channel: QualifiedChannelId,
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
    pub level: Option<Level>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteDelete {
    pub channel: QualifiedChannelId,
    pub id: ChannelInviteId,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteList {
    #[serde(default)]
    pub channel: Option<QualifiedChannelId>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelRefreshLevel {
    pub channel: QualifiedChannelId,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ModerationDecide {
    pub channel: QualifiedChannelId,
    pub message: MessageId,
    pub approve: bool,
}

// Contacts
#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ContactRes {
    pub id: Identity,
    pub name: String,
    pub description: String,
    pub shown_name: Option<String>,
    pub shown_description: Option<String>,
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
    pub shown_name: Option<ModifyOption<String>>,
    #[serde(default)]
    pub shown_description: Option<ModifyOption<String>>,
    #[serde(default)]
    pub portrait: Option<ModifyOption<crate::interface::shared::Portrait>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ContactDelete {
    pub id: Identity,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MemberInfoRes {
    pub name: Option<String>,
    pub description: Option<String>,
    pub portrait: Option<crate::interface::shared::FileRef>,
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
    pub blocks: Vec<MessageBlock>,
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
    ChannelRefreshLevel(ChannelRefreshLevel) => Level,
    ModerationDecide(ModerationDecide) =>(),
    ContactList(ContactList) => Vec < ContactRes >,
    ContactModify(ContactModify) => ContactRes,
    ContactDelete(ContactDelete) =>(),
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
    pub blocks: Vec<MessageBlock>,
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
    pub blocks: Vec<MessageBlock>,
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

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ModerationEntry {
    pub seq: u64,
    pub state: crate::interface::shared::ModerationState,
    pub received: Timestamp,
    pub sender: Identity,
    pub client_id: Option<MessageClientId>,
    pub id: MessageId,
    pub rel: crate::interface::shared::MessageRel,
    pub blocks: Vec<MessageBlock>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModerationCursor {
    Latest,
    Before(u64),
    After(u64),
}

pub const MODERATION_PAGE_LIMIT: u32 = 50;
const MODERATION_CURSOR_LATEST: &str = "latest";
const MODERATION_CURSOR_BEFORE: &str = "before";
const MODERATION_CURSOR_AFTER: &str = "after";

pub fn moderation_path_segments(channel: &QualifiedChannelId, cursor: ModerationCursor) -> Vec<String> {
    let mut out = vec![channel.identity.to_string(), channel.channel.0.to_string()];
    match cursor {
        ModerationCursor::Latest => out.push(MODERATION_CURSOR_LATEST.to_string()),
        ModerationCursor::Before(seq) => {
            out.push(MODERATION_CURSOR_BEFORE.to_string());
            out.push(seq.to_string());
        },
        ModerationCursor::After(seq) => {
            out.push(MODERATION_CURSOR_AFTER.to_string());
            out.push(seq.to_string());
        },
    }
    return out;
}

pub fn moderation_path_parse<
    'a,
>(parts: &mut dyn Iterator<Item = Cow<'a, str>>) -> Result<(QualifiedChannelId, ModerationCursor), String> {
    let channel = QualifiedChannelId {
        identity: confirm_path_element(parts, "channel identity")?,
        channel: ChannelId(confirm_path_element(parts, "channel")?),
    };
    let kind: String = confirm_path_element(parts, "cursor kind")?;
    let cursor = match kind.as_str() {
        MODERATION_CURSOR_LATEST => ModerationCursor::Latest,
        MODERATION_CURSOR_BEFORE => ModerationCursor::Before(confirm_path_element(parts, "cursor seq")?),
        MODERATION_CURSOR_AFTER => ModerationCursor::After(confirm_path_element(parts, "cursor seq")?),
        other => return Err(format!("Unknown moderation cursor kind [{}]", other)),
    };
    return Ok((channel, cursor));
}

pub struct GetModeration {
    pub channel: QualifiedChannelId,
    pub cursor: ModerationCursor,
}

const PATH_PREFIX_MODERATION: &str = "moderation";

impl PathReqTrait for GetModeration {
    type Resp = Vec<ModerationEntry>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_MODERATION)?;
        let (channel, cursor) = moderation_path_parse(&mut parts)?;
        confirm_path_empty(&mut parts)?;
        return Ok(GetModeration {
            channel: channel,
            cursor: cursor,
        });
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [PATH_PREFIX_MODERATION.to_string()]
                .into_iter()
                .chain(moderation_path_segments(&self.channel, self.cursor)),
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

pub struct GetChannelFile {
    pub channel: QualifiedChannelId,
    pub file: QualifiedFileId,
    pub thumbnail: bool,
}

const PATH_PREFIX_CHANNEL_FILE: &str = "channel_file";
const CHANNEL_FILE_VARIANT_ORIGINAL: &str = "o";
const CHANNEL_FILE_VARIANT_THUMBNAIL: &str = "t";

impl PathReqTrait for GetChannelFile {
    type Resp = Vec<u8>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_CHANNEL_FILE)?;
        let variant: String = confirm_path_element(&mut parts, "variant")?;
        let thumbnail = match variant.as_str() {
            CHANNEL_FILE_VARIANT_ORIGINAL => false,
            CHANNEL_FILE_VARIANT_THUMBNAIL => true,
            _ => return Err(format!("Unknown file variant [{}]", variant)),
        };
        let out = GetChannelFile {
            channel: QualifiedChannelId {
                identity: confirm_path_element(&mut parts, "channel identity")?,
                channel: ChannelId(confirm_path_element(&mut parts, "channel")?),
            },
            file: QualifiedFileId {
                identity: confirm_path_element(&mut parts, "file identity")?,
                hash: confirm_path_element(&mut parts, "file hash")?,
            },
            thumbnail: thumbnail,
        };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [
                PATH_PREFIX_CHANNEL_FILE.to_string(),
                if self.thumbnail {
                    CHANNEL_FILE_VARIANT_THUMBNAIL
                } else {
                    CHANNEL_FILE_VARIANT_ORIGINAL
                }.to_string(),
                self.channel.identity.to_string(),
                self.channel.channel.0.to_string(),
                self.file.identity.to_string(),
                self.file.hash.to_string(),
            ],
        );
    }
}

pub const PATH_FILE_UPLOAD: &str = "file_upload";

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FileUploadRes {
    pub hash: crate::interface::shared::FileHash,
    pub size: u64,
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

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CalendarMonth {
    pub year: i16,
    pub month: i8,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CalendarToken {
    pub latest: Option<u64>,
    pub floor: Option<u64>,
}

const CALENDAR_TOKEN_NONE: &str = "_";

impl std::fmt::Display for CalendarToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let part = |v: Option<u64>| v.map(|v| v.to_string()).unwrap_or_else(|| CALENDAR_TOKEN_NONE.to_string());
        return write!(f, "{}-{}", part(self.latest), part(self.floor));
    }
}

impl FromStr for CalendarToken {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((latest, floor)) = s.split_once("-") else {
            return Err(format!("Calendar token missing separator"));
        };
        let part = |v: &str| -> Result<Option<u64>, String> {
            if v == CALENDAR_TOKEN_NONE {
                return Ok(None);
            }
            return Ok(Some(u64::from_str(v).map_err(|e| format!("Invalid calendar token part [{}]: {}", v, e))?));
        };
        return Ok(CalendarToken {
            latest: part(latest)?,
            floor: part(floor)?,
        });
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CalendarMessage {
    pub original_id: QualifiedMessageId,
    pub original_receive_time: Timestamp,
    pub client_id: Option<MessageClientId>,
    pub message: MessageBody,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CalendarPage {
    pub token: CalendarToken,
    pub latest_activity: Option<ActivityPageOffsetPos>,
    pub messages: Vec<CalendarMessage>,
    pub last: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CalendarPageRes {
    Page(CalendarPage),
    Stale,
}

pub fn calendar_path_segments(
    channel: &QualifiedChannelId,
    month: CalendarMonth,
    page: usize,
    token: Option<CalendarToken>,
) -> Vec<String> {
    return vec![
        channel.identity.to_string(),
        channel.channel.0.to_string(),
        month.year.to_string(),
        month.month.to_string(),
        page.to_string(),
        token.map(|t| t.to_string()).unwrap_or_else(|| CALENDAR_TOKEN_NONE.to_string()),
    ];
}

pub fn calendar_path_parse<
    'a,
>(
    parts: &mut dyn Iterator<Item = Cow<'a, str>>,
) -> Result<(QualifiedChannelId, CalendarMonth, usize, Option<CalendarToken>), String> {
    let channel = QualifiedChannelId {
        identity: confirm_path_element(parts, "channel identity")?,
        channel: ChannelId(confirm_path_element(parts, "channel")?),
    };
    let month = CalendarMonth {
        year: confirm_path_element(parts, "year")?,
        month: confirm_path_element(parts, "month")?,
    };
    if !(1 ..= 12).contains(&month.month) {
        return Err(format!("Calendar month out of range"));
    }
    let page = confirm_path_element(parts, "page")?;
    let token: String = confirm_path_element(parts, "token")?;
    let token = if token == CALENDAR_TOKEN_NONE {
        None
    } else {
        Some(CalendarToken::from_str(&token)?)
    };
    return Ok((channel, month, page, token));
}

pub struct GetCalendarPage {
    pub channel: QualifiedChannelId,
    pub month: CalendarMonth,
    pub page: usize,
    pub token: Option<CalendarToken>,
}

const PATH_PREFIX_CALENDAR_PAGE: &str = "calendar_page";

impl PathReqTrait for GetCalendarPage {
    type Resp = Option<CalendarPageRes>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_CALENDAR_PAGE)?;
        let (channel, month, page, token) = calendar_path_parse(&mut parts)?;
        confirm_path_empty(&mut parts)?;
        return Ok(GetCalendarPage {
            channel: channel,
            month: month,
            page: page,
            token: token,
        });
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [PATH_PREFIX_CALENDAR_PAGE.to_string()]
                .into_iter()
                .chain(calendar_path_segments(&self.channel, self.month, self.page, self.token)),
        );
    }
}

pub struct GetChannelMemberInfo {
    pub channel: QualifiedChannelId,
    pub member: Identity,
}

const PATH_PREFIX_CHANNEL_MEMBER_INFO: &str = "channel_member_info";

impl PathReqTrait for GetChannelMemberInfo {
    type Resp = Option<MemberInfoRes>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_CHANNEL_MEMBER_INFO)?;
        let out = GetChannelMemberInfo {
            channel: QualifiedChannelId {
                identity: confirm_path_element(&mut parts, "channel identity")?,
                channel: ChannelId(confirm_path_element(&mut parts, "channel")?),
            },
            member: confirm_path_element(&mut parts, "member")?,
        };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [
                PATH_PREFIX_CHANNEL_MEMBER_INFO.to_string(),
                self.channel.identity.to_string(),
                self.channel.channel.0.to_string(),
                self.member.to_string(),
            ],
        );
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ContactChannel {
    pub channel: QualifiedChannelId,
    pub own_identity: Identity,
}

pub struct GetContactChannels {
    pub identity: Identity,
}

const PATH_PREFIX_CONTACT_CHANNELS: &str = "contact_channels";

impl PathReqTrait for GetContactChannels {
    type Resp = Vec<ContactChannel>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_CONTACT_CHANNELS)?;
        let out = GetContactChannels { identity: confirm_path_element(&mut parts, "identity")? };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path([PATH_PREFIX_CONTACT_CHANNELS.to_string(), self.identity.to_string()]);
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ContactPost {
    pub own_identity: Identity,
    pub original_id: QualifiedMessageId,
    pub original_receive_time: Timestamp,
    pub message: MessageBody,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ContactPostsRes {
    pub posts: Vec<ContactPost>,
    pub last: bool,
}

pub const CONTACT_POSTS_PAGE_SIZE: usize = 50;

pub struct GetContactPosts {
    pub identity: Identity,
    pub page: usize,
}

const PATH_PREFIX_CONTACT_POSTS: &str = "contact_posts";

impl PathReqTrait for GetContactPosts {
    type Resp = ContactPostsRes;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_CONTACT_POSTS)?;
        let out = GetContactPosts {
            identity: confirm_path_element(&mut parts, "identity")?,
            page: confirm_path_element(&mut parts, "page")?,
        };
        confirm_path_empty(&mut parts)?;
        return Ok(out);
    }

    fn serialize_path(&self) -> String {
        return serialize_path(
            [PATH_PREFIX_CONTACT_POSTS.to_string(), self.identity.to_string(), self.page.to_string()],
        );
    }
}
