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
    spaghettinuum::interface::identity::Identity,
};

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Notification {
    pub channel: QualifiedChannelId,
    pub offset: ActivityOffset,
    pub text: String,
    pub files: usize,
    pub extensions: usize,
    pub deleted: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct OutboxChange {
    pub channel: QualifiedChannelId,
    pub sender: Identity,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum WsEvent {
    Activity(Notification),
    Outbox(OutboxChange),
}
