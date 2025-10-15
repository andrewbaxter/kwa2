use {
    glove::reqresp,
    spaghettinuum::interface::{
        stored::identity::Identity,
        config::identity::LocalIdentitySecret,
    },
    super::shared::ChannelId,
};

mod c2sv1t {
    use {
        crate::interface::shared::{
            ChannelId,
            InvitationToken,
            Message,
            MessageBody,
            MessageId,
        },
        chrono::{
            DateTime,
            Utc,
        },
        schemars::JsonSchema,
        serde::{
            Deserialize,
            Serialize,
        },
        spaghettinuum::interface::stored::identity::Identity,
    };

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct LoginByUser {
        pub id: String,
        pub password: String,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct Logout {}

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct IdentityCreate {
        pub idem: Option<String>,
        pub descriptor: String,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct IdentityRes {
        pub id: Identity,
        pub descriptor: String,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct IdentityModify {
        pub id: Identity,
        #[serde(default)]
        pub descriptor: Option<String>,
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
    pub struct IdentitySetLocalKey {
        pub id: Identity,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct IdentityDeleteLocalKey {
        pub id: Identity,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct IdentityGetLocalKey {
        pub id: Identity,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct IdentityList {}

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct ChannelCreate {
        pub idem: Option<String>,
        pub id: Identity,
        pub descriptor: String,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct ChannelRes {
        pub id: ChannelId,
        pub descriptor: String,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct ChannelJoin {}

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct ChannelModify {
        pub id: ChannelId,
        #[serde(default)]
        pub descriptor: Option<String>,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct ChannelDelete {
        pub id: ChannelId,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct ChannelGet {
        pub id: ChannelId,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct ChannelList {}

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct InvitationCreate {
        #[serde(default)]
        pub idem: Option<String>,
        pub descriptor: String,
        #[serde(default)]
        pub accept_decriptor: String,
        #[serde(default)]
        pub channel: Option<ChannelId>,
        #[serde(default)]
        pub identity: Option<Identity>,
        #[serde(default)]
        pub single_use: bool,
        #[serde(default)]
        pub expiry: Option<DateTime<Utc>>,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct InvitationRes {
        pub id: ChannelId,
        pub descriptor: String,
        pub token: InvitationToken,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct InvitationModify {
        pub id: InvitationToken,
        #[serde(default)]
        pub descriptor: Option<String>,
        #[serde(default)]
        pub accept_decriptor: Option<String>,
        #[serde(default)]
        pub channel: Option<ChannelId>,
        #[serde(default)]
        pub identity: Option<Identity>,
        #[serde(default)]
        pub single_use: Option<bool>,
        #[serde(default)]
        pub expiry: Option<DateTime<Utc>>,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct InvitationDelete {
        pub id: InvitationToken,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct InvitationList {}

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
}

reqresp!(pub c2sv1 {
    LoginByUser(c2sv1t::LoginByUser) =>(),
    Logout(c2sv1t::Logout) =>(),
    IdentityCreate(c2sv1t::IdentityCreate) => Identity,
    IdentityUpdate(c2sv1t::IdentityModify) =>(),
    IdentityDelete(c2sv1t::IdentityDelete) =>(),
    IdentityGet(c2sv1t::IdentityGet) => c2sv1t:: IdentityRes,
    // TODO version secret?
    IdentityGetLocalKey(c2sv1t::IdentityGetLocalKey) => LocalIdentitySecret,
    IdentityList(c2sv1t::IdentityList) => Vec < c2sv1t:: IdentityRes >,
    ChannelCreate(c2sv1t::ChannelCreate) => ChannelId,
    ChannelJoin(c2sv1t::ChannelJoin) =>(),
    ChannelUpdate(c2sv1t::ChannelModify) =>(),
    ChannelDelete(c2sv1t::ChannelDelete) =>(),
    ChannelGet(c2sv1t::ChannelGet) => c2sv1t:: ChannelRes,
    ChannelList(c2sv1t::ChannelList) => Vec < c2sv1t:: ChannelRes >,
    InvitationCreate(c2sv1t::InvitationCreate) => c2sv1t:: InvitationRes,
    InvitationDelete(c2sv1t::InvitationDelete) =>(),
    InvitationList(c2sv1t::InvitationList) => Vec < c2sv1t:: InvitationRes >,
    MemberAdd(c2sv1t::MemberAdd) =>(),
    MemberDelete(c2sv1t::MemberDelete) =>(),
    MemberList(c2sv1t::MemberList) => Vec < Identity >,
    MessagePush(c2sv1t::MessagePush) =>(),
    MessageLastPage(c2sv1t::MessageLastPage) => c2sv1t:: MessageLastPageRes,
    MessagePageContaining(c2sv1t::MessagePageContaining) => c2sv1t:: MessagePageContainingRes,
    MessageGetPage(c2sv1t::MessageGetPage) => c2sv1t:: MessageGetPageRes,
    MessageDelete(c2sv1t::MessageDelete) =>(),
});
