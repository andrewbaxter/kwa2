use {
    crate::interface::shared::{
        QualifiedMessageId,
        QualifiedChannelId,
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

impl KwaUrl {
    pub fn to_string(&self) -> String {
        let suffix;
        match self {
            KwaUrl::IdentityInvite(u) => {
                suffix = format!("{}?{}", KWA_URL_PREFIX_IDENTITY_INVITE, serde_urlencoded::to_string(u).unwrap());
            },
            KwaUrl::ChannelInvite(u) => {
                suffix = format!("{}?{}", KWA_URL_PREFIX_CHANNEL_INVITE, serde_urlencoded::to_string(u).unwrap());
            },
            KwaUrl::Channel(u) => {
                suffix = format!("{}?{}", KWA_URL_PREFIX_CHANNEL, serde_urlencoded::to_string(u).unwrap());
            },
            KwaUrl::Message(u) => {
                suffix = format!("{}?{}", KWA_URL_PREFIX_MESSAGE, serde_urlencoded::to_string(u).unwrap());
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
                return Ok(
                    KwaUrl::IdentityInvite(
                        serde_urlencoded::from_str(v).map_err(|e| format!("[{}] has invalid options: {}", s, e))?,
                    ),
                );
            },
            KWA_URL_PREFIX_CHANNEL_INVITE => {
                return Ok(
                    KwaUrl::ChannelInvite(
                        serde_urlencoded::from_str(v).map_err(|e| format!("[{}] has invalid options: {}", s, e))?,
                    ),
                );
            },
            KWA_URL_PREFIX_CHANNEL => {
                return Ok(
                    KwaUrl::Channel(
                        serde_urlencoded::from_str(v).map_err(|e| format!("[{}] has invalid options: {}", s, e))?,
                    ),
                );
            },
            KWA_URL_PREFIX_MESSAGE => {
                return Ok(
                    KwaUrl::Message(
                        serde_urlencoded::from_str(v).map_err(|e| format!("[{}] has invalid options: {}", s, e))?,
                    ),
                );
            },
            _ => {
                return Err(format!("Unrecognized kwa url prefix [{}]", k));
            },
        }
    }
}
