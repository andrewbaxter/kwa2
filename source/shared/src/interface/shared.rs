use {
    jiff::Timestamp,
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    spaghettinuum::{
        byteszb32::BytesZb32,
        interface::identity::{
            Identity,
            IdentityPublicKey,
        },
        jsonsig::JsonSignature,
        sigdomain::{
            SigDomain,
            SigDomained,
        },
    },
};

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Portrait {
    pub mime_type: String,
    pub data: BytesZb32,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FileHash(pub String);

impl std::fmt::Display for FileHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return self.0.fmt(f);
    }
}

impl std::str::FromStr for FileHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 64 || !s.bytes().all(|b| matches!(b, b'0'..= b'9' | b'a'..= b'f')) {
            return Err(format!("File hash must be 64 lowercase hex characters"));
        }
        return Ok(FileHash(s.to_string()));
    }
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct FileRef {
    pub hash: FileHash,
    pub mime_type: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum JoinPortrait {
    FromInvitation,
    Upload(Portrait),
}

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
pub struct MessageClientId(pub String);

impl MessageClientId {
    pub fn from_timestamp(t: Timestamp) -> Self {
        return Self(t.to_string());
    }
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
pub struct QualifiedFileId {
    pub identity: Identity,
    pub hash: FileHash,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageBlockFile {
    pub file: QualifiedFileId,
    pub mime_type: String,
    pub size: u64,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MessageBlockExtension {
    pub key: String,
    pub data: serde_json::Value,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MessageBlock {
    Text(String),
    File(MessageBlockFile),
    Extension(MessageBlockExtension),
}

pub fn mime_is_media(mime_type: &str) -> bool {
    return mime_type.starts_with("image/") || mime_type.starts_with("video/");
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
    pub channel: QualifiedChannelId,
    pub client_id: Option<MessageClientId>,
    pub id: MessageId,
    pub rel: MessageRel,
    pub blocks: Vec<MessageBlock>,
}

impl SigDomained for MessageBody {
    const DOMAIN: SigDomain = SigDomain::App("kwa/message_body");
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Message(pub JsonSignature<MessageBody, IdentityPublicKey>);
