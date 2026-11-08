use {
    crate::{
        db,
        dbutil::tx,
    },
    deadpool_sqlite::Pool,
    loga::ResultContext,
    rusqlite::Transaction,
};

pub const SNAP_PAGE_MAX_AGE_SECS: u32 = 3600;

#[derive(Clone, Copy)]
pub enum CachePolicy {
    NoStore,
    MaxAge(u32),
    Immutable,
}

impl CachePolicy {
    pub fn header_value(&self) -> String {
        match self {
            CachePolicy::NoStore => return "no-store".to_string(),
            CachePolicy::MaxAge(secs) => return format!("public, max-age={}", secs),
            CachePolicy::Immutable => return "public, max-age=31536000, immutable".to_string(),
        }
    }

    pub fn only_if(self, cond: bool) -> CachePolicy {
        if cond {
            return self;
        } else {
            return CachePolicy::NoStore;
        }
    }
}

pub struct Cached<T> {
    pub value: T,
    pub policy: CachePolicy,
}

impl<T> Cached<T> {
    pub fn no_store(value: T) -> Self {
        return Cached {
            value: value,
            policy: CachePolicy::NoStore,
        };
    }
}

pub enum S2sError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Internal(loga::Error),
}

impl From<loga::Error> for S2sError {
    fn from(e: loga::Error) -> Self {
        return S2sError::Internal(e);
    }
}

pub fn not_found<T>(v: Option<T>, message: &str) -> Result<T, S2sError> {
    return v.ok_or_else(|| S2sError::NotFound(message.to_string()));
}

pub async fn s2s_tx<
    O: 'static + Send + Sync,
    F: 'static + Send + for<'b, 't> FnOnce(&'b mut db::Db<Transaction<'t>>) -> Result<O, S2sError>,
>(pool: &Pool, what: &str, cb: F) -> Result<O, S2sError> {
    return tx(pool, move |db_tx| {
        return Ok(cb(db_tx));
    }).await.context(format!("Error {}", what))?;
}
