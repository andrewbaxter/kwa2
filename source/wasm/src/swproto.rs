use {
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::wire::c2s,
};

#[derive(Serialize, Deserialize)]
pub enum FromSw {
    Reload,
    Notification(c2s::Notification),
}
