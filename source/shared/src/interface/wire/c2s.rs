use {
    crate::interface::wire::shared::{
        ChannelGroupId,
        ChannelInviteId,
        IdentityInviteId,
        Message,
        MessageBody,
        MessageId,
        QualifiedChannelId,
        QualifiedChannelInviteToken,
        QualifiedIdentityInviteToken,
    },
    glove::reqresp,
    jiff::Timestamp,
    schemars::JsonSchema,
    serde::{
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
pub struct Logout {}

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

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageLastPageRes {
    pub page: usize,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessagePageContaining {
    pub channel: QualifiedChannelId,
    pub identity: Identity,
    pub message: MessageId,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessagePageContainingRes {
    pub page: usize,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageGetPage {
    pub channel: QualifiedChannelId,
    pub identity: Identity,
    pub page: usize,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageGetPageRes {
    pub messages: Vec<Message>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageDelete {
    pub channel: QualifiedChannelId,
    pub identity: Identity,
    pub message_id: MessageId,
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
    //.    MessageLastPage(MessageLastPage) => MessageLastPageRes,
    //.    MessagePageContaining(MessagePageContaining) => MessagePageContainingRes,
    //.    MessageGetPage(MessageGetPage) => MessageGetPageRes,
    //.    MessageDelete(MessageDelete) =>(),
    });
