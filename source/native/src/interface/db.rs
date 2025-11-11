use {
    good_ormning_runtime::sqlite::{
        GoodOrmningCustomI64,
        GoodOrmningCustomString,
    },
    shared::interface::shared::{
        AccountId,
        ChannelGroupId,
    },
    spaghettinuum::interface::identity::{
        Identity,
        LocalIdentitySecret,
    },
};

pub struct DbAccountId(pub AccountId);

impl GoodOrmningCustomI64<DbAccountId> for DbAccountId {
    fn to_sql(value: &DbAccountId) -> i64 {
        return i64::from_ne_bytes(value.0.0.to_ne_bytes());
    }

    fn from_sql(value: i64) -> Result<DbAccountId, String> {
        return Ok(DbAccountId(AccountId(u64::from_ne_bytes(value.to_ne_bytes()))));
    }
}

pub struct DbIdentitySecret(pub LocalIdentitySecret);

impl GoodOrmningCustomString<DbIdentitySecret> for DbIdentitySecret {
    fn to_sql<'a>(value: &'a DbIdentitySecret) -> String {
        return serde_json::to_string(&value.0).unwrap();
    }

    fn from_sql(value: String) -> Result<DbIdentitySecret, String> {
        return serde_json::from_str::<LocalIdentitySecret>(&value)
            .map_err(|e| e.to_string())
            .map(|x| DbIdentitySecret(x));
    }
}

pub struct DbIdentity(pub Identity);

impl GoodOrmningCustomString<DbIdentity> for DbIdentity {
    fn to_sql<'a>(value: &'a DbIdentity) -> String {
        return serde_json::to_string(&value.0).unwrap();
    }

    fn from_sql(value: String) -> Result<DbIdentity, String> {
        return serde_json::from_str::<Identity>(&value).map_err(|e| e.to_string()).map(|x| DbIdentity(x));
    }
}

pub struct DbChannelGroupId(pub ChannelGroupId);

impl GoodOrmningCustomI64<DbChannelGroupId> for DbChannelGroupId {
    fn to_sql(value: &DbChannelGroupId) -> i64 {
        return i64::from_ne_bytes(value.0.0.to_ne_bytes());
    }

    fn from_sql(value: i64) -> Result<DbChannelGroupId, String> {
        return Ok(DbChannelGroupId(ChannelGroupId(u64::from_ne_bytes(value.to_ne_bytes()))));
    }
}
