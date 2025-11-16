use {
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::{
        shared::QualifiedChannelId,
        wire::c2s::{
            ActivityOffset,
        },
    },
};

#[derive(Serialize, Deserialize)]
pub struct FromSwNotification {
    pub channel: QualifiedChannelId,
    pub offset: ActivityOffset,
}

#[derive(Serialize, Deserialize)]
pub enum FromSw {
    Reload,
    Notification(FromSwNotification),
}
