use {
    crate::interface::{
        shared::QualifiedChannelId,
        wire::c2s::ActivityOffset,
    },
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Notification {
    pub channel: QualifiedChannelId,
    pub offset: ActivityOffset,
    pub body: String,
}
