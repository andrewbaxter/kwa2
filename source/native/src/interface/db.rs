use {
    good_ormning::runtime::sqlite::{
        GoodOrmningCustomI64,
        GoodOrmningCustomString,
    },
    jiff::Timestamp,
    shared::interface::shared::{
        AccountId,
        ChannelGroupId,
        ChannelId,
    },
    spaghettinuum::interface::identity::{
        Identity,
        IdentitySecret,
    },
};

#[derive(Clone, Copy)]
pub struct DbTimeMs(pub Timestamp);

impl GoodOrmningCustomI64<DbTimeMs> for DbTimeMs {
    fn to_sql(value: &DbTimeMs) -> i64 {
        return value.0.as_millisecond();
    }

    fn from_sql(value: i64) -> Result<DbTimeMs, String> {
        return Ok(DbTimeMs(Timestamp::from_millisecond(value).map_err(|e| e.to_string())?));
    }
}

#[derive(Clone, Copy)]
pub struct DbAccountId(pub AccountId);

impl GoodOrmningCustomI64<DbAccountId> for DbAccountId {
    fn to_sql(value: &DbAccountId) -> i64 {
        return i64::from_ne_bytes(value.0.0.to_ne_bytes());
    }

    fn from_sql(value: i64) -> Result<DbAccountId, String> {
        return Ok(DbAccountId(AccountId(u64::from_ne_bytes(value.to_ne_bytes()))));
    }
}

pub struct DbIdentitySecret(pub IdentitySecret);

impl GoodOrmningCustomString<DbIdentitySecret> for DbIdentitySecret {
    fn to_sql<'a>(value: &'a DbIdentitySecret) -> String {
        return serde_json::to_string(&value.0).unwrap();
    }

    fn from_sql(value: String) -> Result<DbIdentitySecret, String> {
        return serde_json::from_str::<IdentitySecret>(&value)
            .map_err(|e| e.to_string())
            .map(|x| DbIdentitySecret(x));
    }
}

#[derive(Clone, Copy)]
pub struct DbIdentity(pub Identity);

impl GoodOrmningCustomString<DbIdentity> for DbIdentity {
    fn to_sql<'a>(value: &'a DbIdentity) -> String {
        return serde_json::to_string(&value.0).unwrap();
    }

    fn from_sql(value: String) -> Result<DbIdentity, String> {
        return serde_json::from_str::<Identity>(&value).map_err(|e| e.to_string()).map(|x| DbIdentity(x));
    }
}

#[derive(Clone, Copy)]
pub struct DbChannelGroupId(pub ChannelGroupId);

impl GoodOrmningCustomI64<DbChannelGroupId> for DbChannelGroupId {
    fn to_sql(value: &DbChannelGroupId) -> i64 {
        return i64::from_ne_bytes(value.0.0.to_ne_bytes());
    }

    fn from_sql(value: i64) -> Result<DbChannelGroupId, String> {
        return Ok(DbChannelGroupId(ChannelGroupId(u64::from_ne_bytes(value.to_ne_bytes()))));
    }
}

#[derive(Clone)]
pub struct DbChannelId(pub ChannelId);

impl GoodOrmningCustomString<DbChannelId> for DbChannelId {
    fn to_sql<'a>(value: &'a DbChannelId) -> String {
        return value.0.0.to_string();
    }

    fn from_sql(value: String) -> Result<DbChannelId, String> {
        return value.parse::<u64>().map(|v| DbChannelId(ChannelId(v))).map_err(|e| e.to_string());
    }
}
