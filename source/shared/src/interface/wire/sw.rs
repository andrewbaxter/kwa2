use {
    crate::interface::{
        shared::QualifiedChannelId,
        wire::c2s::ActivityOffset,
    },
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FromSwNotification {
    pub channel: QualifiedChannelId,
    pub offset: ActivityOffset,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FromSwReload {
    pub reason: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FromSw {
    Reload(FromSwReload),
    Notification(FromSwNotification),
}
