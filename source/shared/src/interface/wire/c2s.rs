use {
    super::shared::ChannelId,
    crate::interface::wire::shared::{
        ChannelInvitationToken,
        IdentityInvitationToken,
        InternalChannelGroupId,
        InternalChannelId,
        InternalIdentityId,
        Message,
        MessageBody,
        MessageId,
    },
    chrono::{
        DateTime,
        Utc,
    },
    glove::reqresp,
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

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityRes {
    pub internal_id: InternalIdentityId,
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
    pub memo_long: String,
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

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupRes {
    pub internal_id: InternalChannelGroupId,
    pub idem: Option<String>,
    pub memo_short: String,
    pub memo_long: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupModify {
    pub internal_id: InternalChannelGroupId,
    #[serde(default)]
    pub memo_short: Option<String>,
    #[serde(default)]
    pub memo_long: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupDelete {
    pub internal_id: InternalChannelGroupId,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupGet {
    pub internal_id: InternalChannelGroupId,
}

// # Channel
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelCreate {
    pub idem: Option<String>,
    pub id: Identity,
    pub group: Option<InternalChannelGroupId>,
    pub memo_short: String,
    pub memo_long: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelRes {
    pub identity: Identity,
    pub id: ChannelId,
    pub idem: Option<String>,
    pub group: Option<InternalChannelGroupId>,
    pub memo_short: String,
    pub memo_long: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelList;
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelJoin;

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelModify {
    pub internal_id: InternalChannelId,
    #[serde(default)]
    pub group: Option<ModifyOption<InternalChannelGroupId>>,
    #[serde(default)]
    pub memo_short: Option<String>,
    #[serde(default)]
    pub memo_long: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelDelete {
    pub internal_id: InternalChannelId,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGet {
    pub internal_id: InternalChannelId,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelOrChannelGroupTree;

// # Identity invitation
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInvitationCreate {
    #[serde(default)]
    pub idem: Option<String>,
    pub memo_short: String,
    pub memo_long: String,
    pub identity: Identity,
    #[serde(default)]
    pub single_use: bool,
    #[serde(default)]
    pub expiry: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInvitationRes {
    pub id: ChannelId,
    pub memo_short: String,
    pub memo_long: String,
    pub identity: Identity,
    pub single_use: bool,
    pub expiry: DateTime<Utc>,
    pub token: IdentityInvitationToken,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInvitationModify {
    pub id: IdentityInvitationToken,
    #[serde(default)]
    pub memo_short: Option<String>,
    #[serde(default)]
    pub memo_long: String,
    #[serde(default)]
    pub single_use: Option<bool>,
    #[serde(default)]
    pub expiry: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInvitationDelete {
    pub id: IdentityInvitationToken,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInvitationList {}

// # Channel invitation
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInvitationCreate {
    #[serde(default)]
    pub idem: Option<String>,
    pub memo_short: String,
    pub memo_long: String,
    #[serde(default)]
    pub single_use: bool,
    #[serde(default)]
    pub expiry: Option<ModifyOption<DateTime<Utc>>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInvitationRes {
    pub id: ChannelId,
    pub memo_short: String,
    pub memo_long: String,
    pub token: ChannelInvitationToken,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInvitationModify {
    pub id: ChannelInvitationToken,
    #[serde(default)]
    pub memo_short: Option<String>,
    #[serde(default)]
    pub memo_long: String,
    #[serde(default)]
    pub single_use: Option<bool>,
    #[serde(default)]
    pub expiry: Option<ModifyOption<DateTime<Utc>>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInvitationDelete {
    pub id: ChannelInvitationToken,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInvitationList;

// Channel member
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MemberAdd {
    pub channel: ChannelId,
    pub identity: Identity,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MemberDelete {
    pub channel: ChannelId,
    pub identity: Identity,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MemberList {}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessagePush {
    pub idem: Option<String>,
    pub channel: ChannelId,
    pub identity: Identity,
    pub body: MessageBody,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageLastPage {
    pub channel: ChannelId,
    pub identity: Identity,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageLastPageRes {
    pub page: usize,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessagePageContaining {
    pub channel: ChannelId,
    pub identity: Identity,
    pub message: MessageId,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessagePageContainingRes {
    pub page: usize,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageGetPage {
    pub channel: ChannelId,
    pub identity: Identity,
    pub page: usize,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageGetPageRes {
    pub messages: Vec<Message>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageDelete {
    pub channel: ChannelId,
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
        //.    IdentityCreate(IdentityCreate) => IdentityRes,
        //.    IdentityModify(IdentityModify) =>(),
        //.    IdentityDelete(IdentityDelete) =>(),
        //.    IdentityGet(IdentityGet) => IdentityRes,
        IdentityList(IdentityList) => Vec < IdentityRes >,
        //.    ChannelCreate(ChannelCreate) => ChannelId,
        //.    ChannelJoin(ChannelJoin) =>(),
        //.    ChannelModify(ChannelModify) =>(),
        //.    ChannelDelete(ChannelDelete) =>(),
        ChannelList(ChannelList) => Vec < ChannelRes >,
        //.    ChannelGet(ChannelGet) => ChannelRes,
        //.    ChannelGroupCreate(ChannelGroupCreate) => ChannelGroupRes,
        //.    ChannelGroupModify(ChannelGroupModify) =>(),
        //.    ChannelGroupDelete(ChannelGroupDelete) =>(),
        //.    ChannelGroupGet(ChannelGroupGet) => ChannelGroupRes,
        ChannelOrChannelGroupTree(ChannelOrChannelGroupTree) => Vec < ChannelOrChannelGroup >,
    //.    IdentityInvitationCreate(IdentityInvitationCreate) => IdentityInvitationRes,
    //.    IdentityInvitationModify(IdentityInvitationModify) =>(),
    //.    IdentityInvitationDelete(IdentityInvitationDelete) =>(),
    //.    IdentityInvitationList(IdentityInvitationList) => Vec < IdentityInvitationRes >,
    //.    ChannelInvitationCreate(ChannelInvitationCreate) => ChannelInvitationRes,
    //.    ChannelInvitationModify(ChannelInvitationModify) =>(),
    //.    ChannelInvitationDelete(ChannelInvitationDelete) =>(),
    //.    ChannelInvitationList(ChannelInvitationList) => Vec < ChannelInvitationRes >,
    //.    MemberAdd(MemberAdd) =>(),
    //.    MemberDelete(MemberDelete) =>(),
    //.    MemberList(MemberList) => Vec < Identity >,
    //.    MessagePush(MessagePush) =>(),
    //.    MessageLastPage(MessageLastPage) => MessageLastPageRes,
    //.    MessagePageContaining(MessagePageContaining) => MessagePageContainingRes,
    //.    MessageGetPage(MessageGetPage) => MessageGetPageRes,
    //.    MessageDelete(MessageDelete) =>(),
    });
