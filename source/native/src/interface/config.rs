use {
    schemars::JsonSchema,
    serde::{
        Deserialize,
        Serialize,
    },
    ts_rs::TS,
};

#[derive(Serialize, Deserialize, Clone, JsonSchema, TS)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct OidcConfig {
    pub provider_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
}
