use {
    crate::{
        db,
        dbutil::tx,
        interface::s2s::{
            S2sErrorBody,
            S2sErrorKind,
        },
    },
    deadpool_sqlite::Pool,
    http::StatusCode,
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

impl S2sError {
    pub fn status(&self) -> StatusCode {
        match self {
            S2sError::BadRequest(_) => return StatusCode::BAD_REQUEST,
            S2sError::Unauthorized(_) => return StatusCode::UNAUTHORIZED,
            S2sError::Forbidden(_) => return StatusCode::FORBIDDEN,
            S2sError::NotFound(_) => return StatusCode::NOT_FOUND,
            S2sError::Internal(_) => return StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn body(&self) -> S2sErrorBody {
        match self {
            S2sError::BadRequest(m) => return S2sErrorBody {
                kind: S2sErrorKind::BadRequest,
                message: m.clone(),
            },
            S2sError::Unauthorized(m) => return S2sErrorBody {
                kind: S2sErrorKind::Unauthorized,
                message: m.clone(),
            },
            S2sError::Forbidden(m) => return S2sErrorBody {
                kind: S2sErrorKind::Forbidden,
                message: m.clone(),
            },
            S2sError::NotFound(m) => return S2sErrorBody {
                kind: S2sErrorKind::NotFound,
                message: m.clone(),
            },
            S2sError::Internal(_) => return S2sErrorBody {
                kind: S2sErrorKind::Internal,
                message: "Internal error".to_string(),
            },
        }
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
