use {
    serde::{
        Deserialize,
        Serialize,
    },
    shared::interface::shared::OutboxMessageId,
};

#[derive(Serialize, Deserialize)]
pub struct OutboxMessage {
    pub idem: OutboxMessageId,
    pub body: String,
}
