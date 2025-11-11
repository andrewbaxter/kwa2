use {
    glove::reqresp,
    shared::interface::shared::{
        ChannelId,
    },
};

// POST only.
//
// Page and file fetching are handled via separate endpoints (GET).
pub mod s2sv1t {
    use {
        schemars::JsonSchema,
        serde::{
            Deserialize,
            Serialize,
        },
        shared::interface::shared::{
            ChannelId,
        },
        spaghettinuum::{
            byteszb32::BytesZb32,
            interface::{
                identity::Identity,
                signature::Signature,
            },
        },
    };

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct StartIdentify {}

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct StartIdentifyRes {
        pub challenge: BytesZb32,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct Identify {
        pub challenge: Signature<BytesZb32>,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct Notify {
        pub owner: Identity,
        pub channel: ChannelId,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub enum ChannelOrIdentity {
        Channel(ChannelId),
        Identity(Identity),
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct Join {
        pub target: ChannelOrIdentity,
        pub token: InvitationToken,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct GetLastPageRes {
        pub page_size: usize,
        pub count: usize,
    }
}

reqresp!(pub s2sv1 {
    StartIdentify(s2sv1t::StartIdentify) => s2sv1t:: StartIdentifyRes,
    Identify(s2sv1t::Identify) => String,
    Notify(s2sv1t::Notify) =>(),
    Join(s2sv1t::Join) => ChannelId,
});
