use {
    glove::reqresp,
    crate::interface::shared::ChannelId,
};

pub mod s2sv1t {
    use {
        crate::interface::shared::InvitationToken,
        schemars::JsonSchema,
        serde::{
            Deserialize,
            Serialize,
        },
        spaghettinuum::interface::{
            stored::identity::Identity,
            wire::api::publish::v1::JsonSignature,
        },
    };

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct StartIdentify {}

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct Identify {
        pub identity: Identity,
        pub challenge: JsonSignature<String, Identity>,
    }

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct Notify {}

    #[derive(Serialize, Deserialize, JsonSchema)]
    #[serde(rename_all = "snake_case", deny_unknown_fields)]
    pub struct Join {
        pub token: InvitationToken,
    }
}

reqresp!(pub s2sv1 {
    StartIdentify(s2sv1t::StartIdentify) =>(),
    Identify(s2sv1t::Identify) =>(),
    Notify(s2sv1t::Notify) =>(),
    Join(s2sv1t::Join) => ChannelId,
});
