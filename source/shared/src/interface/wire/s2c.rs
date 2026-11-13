use {
    crate::interface::{
        shared::{
            QualifiedChannelId,
            QualifiedMessageId,
        },
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
    pub events: usize,
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
pub struct ModerationChange {
    pub channel: QualifiedChannelId,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum WsEvent {
    Activity(Notification),
    Outbox(OutboxChange),
    Moderation(ModerationChange),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReminderNotification {
    pub message: QualifiedMessageId,
    pub text: String,
    pub start: jiff::Timestamp,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PushPayload {
    Activity(Notification),
    Reminder(ReminderNotification),
}
