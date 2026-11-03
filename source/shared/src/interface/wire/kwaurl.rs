use {
    crate::interface::shared::{
        ChannelId,
        MessageId,
        QualifiedChannelId,
        QualifiedMessageId,
    },
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    spaghettinuum::interface::identity::Identity,
    std::str::FromStr,
};

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct KwaUrlIdentityInvite {
    pub identity: Identity,
    pub code: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct KwaUrlChannelInvite {
    pub channel: QualifiedChannelId,
    pub code: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum KwaUrl {
    IdentityInvite(KwaUrlIdentityInvite),
    ChannelInvite(KwaUrlChannelInvite),
    Channel(QualifiedChannelId),
    Message(QualifiedMessageId),
}

const KWA_URL_PREFIX_IDENTITY_INVITE: &str = "in/i";
const KWA_URL_PREFIX_CHANNEL_INVITE: &str = "in/c";
const KWA_URL_PREFIX_CHANNEL: &str = "c";
const KWA_URL_PREFIX_MESSAGE: &str = "m";
const KWA_URL_PREFIX0: &str = "kwa:";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct WireIdentityInvite {
    identity: String,
    code: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct WireChannelInvite {
    identity: String,
    channel: u64,
    code: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct WireChannel {
    identity: String,
    channel: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
struct WireMessage {
    identity: String,
    channel: u64,
    sender: String,
    unique: u64,
}

fn parse_identity(s: &str, v: &str) -> Result<Identity, String> {
    return Identity::from_str(v).map_err(|e| format!("[{}] has invalid identity [{}]: {}", s, v, e));
}

impl KwaUrl {
    pub fn to_string(&self) -> String {
        let suffix;
        match self {
            KwaUrl::IdentityInvite(u) => {
                suffix =
                    format!(
                        "{}?{}",
                        KWA_URL_PREFIX_IDENTITY_INVITE,
                        serde_urlencoded::to_string(WireIdentityInvite {
                            identity: u.identity.to_string(),
                            code: u.code.clone(),
                        }).unwrap()
                    );
            },
            KwaUrl::ChannelInvite(u) => {
                suffix =
                    format!(
                        "{}?{}",
                        KWA_URL_PREFIX_CHANNEL_INVITE,
                        serde_urlencoded::to_string(WireChannelInvite {
                            identity: u.channel.identity.to_string(),
                            channel: u.channel.channel.0,
                            code: u.code.clone(),
                        }).unwrap()
                    );
            },
            KwaUrl::Channel(u) => {
                suffix =
                    format!(
                        "{}?{}",
                        KWA_URL_PREFIX_CHANNEL,
                        serde_urlencoded::to_string(WireChannel {
                            identity: u.identity.to_string(),
                            channel: u.channel.0,
                        }).unwrap()
                    );
            },
            KwaUrl::Message(u) => {
                suffix =
                    format!(
                        "{}?{}",
                        KWA_URL_PREFIX_MESSAGE,
                        serde_urlencoded::to_string(WireMessage {
                            identity: u.channel.identity.to_string(),
                            channel: u.channel.channel.0,
                            sender: u.message.identity.to_string(),
                            unique: u.message.unique,
                        }).unwrap()
                    );
            },
        }
        return format!("{}{}", KWA_URL_PREFIX0, suffix);
    }

    pub fn from_string(s: &str) -> Result<KwaUrl, String> {
        let Some(s) = s.strip_prefix(KWA_URL_PREFIX0) else {
            return Err(format!("[{}] is missing prefix [{}]", s, KWA_URL_PREFIX0));
        };
        let Some((k, v)) = s.split_once("?") else {
            return Err(format!("[{}] is missing [?] and suffix", s));
        };
        match k {
            KWA_URL_PREFIX_IDENTITY_INVITE => {
                let w =
                    serde_urlencoded::from_str::<WireIdentityInvite>(
                        v,
                    ).map_err(|e| format!("[{}] has invalid options: {}", s, e))?;
                return Ok(KwaUrl::IdentityInvite(KwaUrlIdentityInvite {
                    identity: parse_identity(s, &w.identity)?,
                    code: w.code,
                }));
            },
            KWA_URL_PREFIX_CHANNEL_INVITE => {
                let w =
                    serde_urlencoded::from_str::<WireChannelInvite>(
                        v,
                    ).map_err(|e| format!("[{}] has invalid options: {}", s, e))?;
                return Ok(KwaUrl::ChannelInvite(KwaUrlChannelInvite {
                    channel: QualifiedChannelId {
                        identity: parse_identity(s, &w.identity)?,
                        channel: ChannelId(w.channel),
                    },
                    code: w.code,
                }));
            },
            KWA_URL_PREFIX_CHANNEL => {
                let w =
                    serde_urlencoded::from_str::<WireChannel>(
                        v,
                    ).map_err(|e| format!("[{}] has invalid options: {}", s, e))?;
                return Ok(KwaUrl::Channel(QualifiedChannelId {
                    identity: parse_identity(s, &w.identity)?,
                    channel: ChannelId(w.channel),
                }));
            },
            KWA_URL_PREFIX_MESSAGE => {
                let w =
                    serde_urlencoded::from_str::<WireMessage>(
                        v,
                    ).map_err(|e| format!("[{}] has invalid options: {}", s, e))?;
                return Ok(KwaUrl::Message(QualifiedMessageId {
                    channel: QualifiedChannelId {
                        identity: parse_identity(s, &w.identity)?,
                        channel: ChannelId(w.channel),
                    },
                    message: MessageId {
                        identity: parse_identity(s, &w.sender)?,
                        unique: w.unique,
                    },
                }));
            },
            _ => {
                return Err(format!("Unrecognized kwa url prefix [{}]", k));
            },
        }
    }
}
