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
        collections::HashMap,
        str::FromStr,
    },
};

#[derive(Serialize, Deserialize, JsonSchema)]
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
    pub memo_short: String,
    pub memo_long: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityRes {
    pub id: Identity,
    pub idem: Option<String>,
    pub memo_short: String,
    pub memo_long: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityModify {
    pub id: Identity,
    #[serde(default)]
    pub memo_short: Option<String>,
    #[serde(default)]
    pub memo_long: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityDelete {
    pub id: Identity,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityList;

// # Channel group
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupCreate {
    pub idem: Option<String>,
    pub memo_short: String,
    pub memo_long: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupRes {
    pub id: ChannelGroupId,
    pub idem: Option<String>,
    pub memo_short: String,
    pub memo_long: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupList;

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupModify {
    pub id: ChannelGroupId,
    #[serde(default)]
    pub memo_short: Option<String>,
    #[serde(default)]
    pub memo_long: Option<String>,
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
    pub memo_short: String,
    pub memo_long: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelRes {
    pub own_identity: Identity,
    pub id: QualifiedChannelId,
    pub idem: Option<String>,
    pub group: Option<ChannelGroupId>,
    pub memo_short: String,
    pub memo_long: String,
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
    pub memo_short: Option<String>,
    #[serde(default)]
    pub memo_long: Option<String>,
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
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelJoinIdentity {
    pub identity: Identity,
    pub sender: Identity,
    pub code: String,
}

// # Identity invitation
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInviteCreate {
    #[serde(default)]
    pub idem: Option<String>,
    pub memo_short: String,
    pub memo_long: String,
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
    pub memo_short: String,
    pub memo_long: String,
    pub single_use: bool,
    pub expiry: Option<Timestamp>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInviteModify {
    pub id: IdentityInviteId,
    #[serde(default)]
    pub memo_short: Option<String>,
    #[serde(default)]
    pub memo_long: Option<String>,
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

// # Channel invitation
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteCreate {
    pub channel: QualifiedChannelId,
    #[serde(default)]
    pub idem: Option<String>,
    pub memo_short: String,
    pub memo_long: String,
    #[serde(default)]
    pub single_use: bool,
    #[serde(default)]
    pub expiry: Option<Timestamp>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteRes {
    pub id: ChannelInviteId,
    pub token: QualifiedChannelInviteToken,
    pub memo_short: String,
    pub memo_long: String,
    pub single_use: bool,
    pub expiry: Option<Timestamp>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteModify {
    pub id: ChannelInviteId,
    #[serde(default)]
    pub memo_short: Option<String>,
    #[serde(default)]
    pub memo_long: Option<String>,
    #[serde(default)]
    pub single_use: Option<bool>,
    #[serde(default)]
    pub expiry: Option<ModifyOption<Timestamp>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteDelete {
    pub id: ChannelInviteId,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteList;

// Channel member
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MemberAdd {
    pub channel: QualifiedChannelId,
    pub identity: Identity,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MemberDelete {
    pub channel: QualifiedChannelId,
    pub identity: Identity,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MemberList {}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessagePush {
    pub client_id: MessageClientId,
    pub channel: QualifiedChannelId,
    pub identity: Identity,
    pub body: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageLastPage {
    pub channel: QualifiedChannelId,
    pub identity: Identity,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessagePageContaining {
    pub channel: QualifiedChannelId,
    pub identity: Identity,
    pub message: MessageId,
}

// Other
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelOrChannelGroupGroup {
    pub group: ChannelGroupRes,
    pub children: Vec<ChannelRes>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ChannelOrChannelGroup {
    Channel(ChannelRes),
    ChannelGroup(ChannelOrChannelGroupGroup),
}

reqresp!(pub proto {
    Logout(Logout) =>(),
    NotificationRegister(NotificationRegister) =>(),
    IdentityCreate(IdentityCreate) => IdentityRes,
    IdentityModify(IdentityModify) => IdentityRes,
    IdentityDelete(IdentityDelete) =>(),
    IdentityList(IdentityList) => Vec < IdentityRes >,
    ChannelGroupCreate(ChannelGroupCreate) => ChannelGroupRes,
    ChannelGroupModify(ChannelGroupModify) => ChannelGroupRes,
    ChannelGroupDelete(ChannelGroupDelete) =>(),
    ChannelGroupList(ChannelGroupList) => Vec < ChannelGroupRes >,
    ChannelCreate(ChannelCreate) => ChannelRes,
    ChannelJoinChannel(ChannelJoinChannel) => ChannelRes,
    ChannelJoinIdentity(ChannelJoinIdentity) => ChannelRes,
    ChannelModify(ChannelModify) => ChannelRes,
    ChannelDelete(ChannelDelete) =>(),
    ChannelList(ChannelList) => Vec < ChannelRes >,
    //.    ChannelGet(ChannelGet) => ChannelRes,
    IdentityInviteCreate(IdentityInviteCreate) => IdentityInviteRes,
    IdentityInviteModify(IdentityInviteModify) => IdentityInviteRes,
    IdentityInviteDelete(IdentityInviteDelete) =>(),
    IdentityInviteList(IdentityInviteList) => Vec < IdentityInviteRes >,
    ChannelInviteCreate(ChannelInviteCreate) => ChannelInviteRes,
    ChannelInviteModify(ChannelInviteModify) => ChannelInviteRes,
    ChannelInviteDelete(ChannelInviteDelete) =>(),
    ChannelInviteList(ChannelInviteList) => Vec < ChannelInviteRes >,
    //.    MemberAdd(MemberAdd) =>(),
    //.    MemberDelete(MemberDelete) =>(),
    //.    MemberList(MemberList) => Vec < Identity >,
    MessagePush(MessagePush) =>(),
});

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SnapOffset(pub usize);

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ActivityOffset(pub usize);

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SnapMessage {
    pub offset: SnapOffset,
    pub original_id: QualifiedMessageId,
    pub original_receive_time: Timestamp,
    pub client_id: Option<MessageClientId>,
    pub message: MessageBody,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SnapPageRes {
    pub offset: SnapOffset,
    pub page_size: usize,
    pub activity_offset: ActivityOffset,
    pub activity_page_size: usize,
    pub messages: Vec<SnapMessage>,
}

pub trait PathReqTrait: Sized {
    type Resp: DeserializeOwned;

    fn deserialize_path(path: &str) -> Result<Self, String>;
    fn serialize_path(&self) -> String;
}

fn serialize_path(x: impl IntoIterator<Item = String>) -> String {
    return x.into_iter().map(|x| format!("/{}", urlencoding::encode(&x))).collect::<Vec<_>>().join("");
}

fn deserialize_path<'a>(x: &'a str) -> impl Iterator<Item = Cow<'a, str>> {
    return x.split("/").map(|x| urlencoding::decode(x).unwrap_or(std::borrow::Cow::Borrowed(x)));
}

fn confirm_path_const<'a>(segs: &mut dyn Iterator<Item = Cow<'a, str>>, want: &str) -> Result<(), String> {
    let next = segs.next();
    let Some(x) = next.as_ref().map(|x| x.as_ref()) else {
        return Err(format!("Path missing segment [{}]", want));
    };
    if x != want {
        return Err(format!("Path missing segment [{}]", want));
    }
    return Ok(());
}

fn confirm_path_empty<'a>(segs: &mut dyn Iterator<Item = Cow<'a, str>>) -> Result<(), String> {
    if segs.next().is_some() {
        return Err(format!("Unrecognized extra path segments"));
    }
    return Ok(());
}

fn confirm_path_element<
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
    pub offset: SnapOffset,
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
        let out = SnapById { id: QualifiedMessageId {
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

pub struct SnapPageContainingTime {
    pub channel: QualifiedChannelId,
    pub time: Timestamp,
}

const PATH_PREFIX_SNAP_PAGE_CONTAINING_TIME: &str = "snap_page_containing_time";

impl PathReqTrait for SnapPageContainingTime {
    type Resp = Option<SnapOffset>;

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

pub struct SnapPage {
    pub channel: QualifiedChannelId,
    pub offset: SnapOffset,
}

const PATH_PREFIX_SNAP_PAGE: &str = "snap_page";

impl PathReqTrait for SnapPage {
    type Resp = Option<SnapPageRes>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_SNAP_PAGE)?;
        let out = SnapPage {
            channel: QualifiedChannelId {
                identity: confirm_path_element(&mut parts, "channel identity")?,
                channel: ChannelId(confirm_path_element(&mut parts, "channel")?),
            },
            offset: SnapOffset(confirm_path_element(&mut parts, "snap offset")?),
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
                self.offset.0.to_string(),
            ],
        );
    }
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ActivityLatestAll;

const PATH_PREFIX_ACTIVITY_LATEST_ALL: &str = "activity_latest_all";

impl PathReqTrait for ActivityLatestAll {
    type Resp = HashMap<QualifiedChannelId, ActivityOffset>;

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
pub struct ActivityPageRes {
    pub offset: ActivityOffset,
    pub messages: Vec<Message>,
}

pub struct ActivityPage {
    pub channel: QualifiedChannelId,
    pub offset: ActivityOffset,
}

const PATH_PREFIX_ACTIVITY_PAGE: &str = "activity_page";

impl PathReqTrait for ActivityPage {
    type Resp = Option<ActivityPageRes>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        let mut parts = deserialize_path(path);
        confirm_path_const(&mut parts, "")?;
        confirm_path_const(&mut parts, PATH_PREFIX_ACTIVITY_PAGE)?;
        let out = ActivityPage {
            channel: QualifiedChannelId {
                identity: confirm_path_element(&mut parts, "channel identity")?,
                channel: ChannelId(confirm_path_element(&mut parts, "channel")?),
            },
            offset: ActivityOffset(confirm_path_element(&mut parts, "offset")?),
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
                self.offset.0.to_string(),
            ],
        );
    }
}
