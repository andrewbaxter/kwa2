use {
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    spaghettinuum::interface::{
        identity::Identity,
        signature::Signature,
    },
};

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AccountId(pub u64);

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelGroupId(pub u64);

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInviteToken(pub String);

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QualifiedIdentityInviteToken {
    pub identity: Identity,
    pub token: IdentityInviteToken,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct IdentityInviteId(pub u64);

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteToken(pub String);

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QualifiedChannelInviteToken {
    pub channel: QualifiedChannelId,
    pub token: ChannelInviteToken,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelInviteId(pub u64);

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ChannelId(pub u64);

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QualifiedChannelId {
    pub identity: Identity,
    pub channel: ChannelId,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageId {
    pub identity: Identity,
    pub unique: u64,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QualifiedMessageId {
    pub channel: QualifiedChannelId,
    pub message: MessageId,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct OutboxMessageId(pub String);

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MessageBlock {
    Text(String),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MessageRel {
    None,
    ReplyTo(MessageId),
    EditOf(MessageId),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageBody {
    pub id: MessageId,
    pub rel: MessageRel,
    //. pub blocks: Vec<MessageBlock>,
    pub body: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Message(pub Signature<MessageBody>);
