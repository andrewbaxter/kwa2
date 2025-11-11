use {
    crate::interface::shared::{
        ChannelGroupId,
        ChannelInviteId,
        IdentityInviteId,
        Message,
        MessageBody,
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
        de::DeserializeOwned,
        Deserialize,
        Serialize,
    },
    spaghettinuum::interface::identity::Identity,
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
pub struct IdentityGet {
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

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupGet {
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
pub struct ChannelGet {
    pub id: QualifiedChannelId,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelJoinChannel {
    pub channel: QualifiedChannelId,
    pub code: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelJoinIdentity {
    pub identity: Identity,
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
    pub idem: Option<String>,
    pub channel: QualifiedChannelId,
    pub identity: Identity,
    pub body: MessageBody,
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
        //.    IdentityGet(IdentityGet) => IdentityRes,
        IdentityList(IdentityList) => Vec < IdentityRes >,
        ChannelGroupCreate(ChannelGroupCreate) => ChannelGroupRes,
        ChannelGroupModify(ChannelGroupModify) => ChannelGroupRes,
        ChannelGroupDelete(ChannelGroupDelete) =>(),
        //.    ChannelGroupGet(ChannelGroupGet) => ChannelGroupRes,
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
    //.    MessagePush(MessagePush) =>(),
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
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct NotificationServerKey;

impl PathReqTrait for NotificationServerKey {
    // b64-encoded key as required by js
    type Resp = String;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        todo!()
    }

    fn serialize_path(&self) -> String {
        todo!()
    }
}

pub struct SnapPageContaining {
    pub id: QualifiedMessageId,
}

impl PathReqTrait for SnapPageContaining {
    type Resp = Option<SnapPageRes>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        todo!()
    }

    fn serialize_path(&self) -> String {
        todo!()
    }
}

pub struct SnapPageContainingTime {
    pub channel: QualifiedChannelId,
    pub time: Timestamp,
}

impl PathReqTrait for SnapPageContainingTime {
    type Resp = Option<SnapPageRes>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        todo!()
    }

    fn serialize_path(&self) -> String {
        todo!()
    }
}

pub struct SnapPage {
    pub channel: QualifiedChannelId,
    pub offset: SnapOffset,
}

impl PathReqTrait for SnapPage {
    type Resp = Option<SnapPageRes>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        todo!()
    }

    fn serialize_path(&self) -> String {
        todo!()
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ActivityPageRes {
    pub offset: ActivityOffset,
    pub messages: Vec<Message>,
}

pub struct ActivityPage {
    pub offset: ActivityOffset,
}

impl PathReqTrait for ActivityPage {
    type Resp = Option<ActivityPageRes>;

    fn deserialize_path(path: &str) -> Result<Self, String> {
        todo!()
    }

    fn serialize_path(&self) -> String {
        todo!()
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Notification {
    pub message: QualifiedMessageId,
    pub body: String,
}
