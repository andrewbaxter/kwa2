use {
    deadpool_sqlite::Pool,
    loga::ResultContext,
    rusqlite::Transaction,
};

pub async fn tx<
    O: 'static + Send + Sync,
    F: 'static + Send + for<'b, 't> FnOnce(&'b mut crate::db::Db<Transaction<'t>>) -> Result<O, loga::Error>,
>(pool: &Pool, cb: F) -> Result<O, loga::Error> {
    let conn = pool.get().await?;
    return Ok(conn.interact(|conn| {
        let mut tx = conn.transaction()?;
        let mut db_tx = crate::db::Db(tx);
        match cb(&mut db_tx) {
            Ok(res) => {
                db_tx.0.commit().context("Failed to commit transaction")?;
                Ok(res)
            },
            Err(e) => {
                let e = e.context("Error during transaction");
                match db_tx.0.rollback().context("Error rolling back transaction due to error") {
                    Err(re) => {
                        return Err(e.also(re));
                    },
                    Ok(_) => {
                        return Err(e);
                    },
                };
            },
        }
    }).await??);
}

pub enum Txr<T> {
    Ok(T),
    Abort,
}

pub async fn abortable_tx<
    O: 'static + Send + Sync,
    F: 'static + Send + for<'b, 't> FnOnce(&'b mut crate::db::Db<Transaction<'t>>) -> Result<Txr<O>, loga::Error>,
>(pool: &Pool, cb: F) -> Result<Option<O>, loga::Error> {
    let conn = pool.get().await?;
    return Ok(conn.interact(|conn| {
        let mut tx = conn.transaction()?;
        let mut db_tx = crate::db::Db(tx);
        match cb(&mut db_tx) {
            Ok(Txr::Ok(res)) => {
                db_tx.0.commit().context("Failed to commit transaction")?;
                Ok(Some(res))
            },
            Ok(Txr::Abort) => {
                match db_tx.0.rollback().context("Error rolling back transaction due to abort") {
                    Err(re) => {
                        return Err(re);
                    },
                    Ok(_) => {
                        return Ok(None);
                    },
                };
            },
            Err(e) => {
                let e = e.context("Error during transaction");
                match db_tx.0.rollback().context("Error rolling back transaction due to error") {
                    Err(re) => {
                        return Err(e.also(re));
                    },
                    Ok(_) => {
                        return Err(e);
                    },
                };
            },
        }
    }).await??);
}
