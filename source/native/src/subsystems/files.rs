use {
    crate::{
        db,
        dbutil::tx,
        fsutil::create_dirs,
        interface::db::{
            DbAccountId,
            DbTimeMs,
        },
        State,
    },
    jiff::Timestamp,
    loga::{
        ea,
        DebugDisplay,
        ResultContext,
    },
    rusqlite::Transaction,
    sha2::{
        Digest,
        Sha256,
    },
    shared::interface::shared::{
        FileHash,
        FileRef,
    },
    std::path::PathBuf,
};

pub fn file_path(state: &State, account: DbAccountId, hash: &FileHash) -> PathBuf {
    return state.files_dir.join(account.0.0.to_string()).join(&hash.0);
}

pub async fn store_file(
    state: &State,
    account: DbAccountId,
    mime_type: String,
    data: Vec<u8>,
) -> Result<FileRef, loga::Error> {
    let hash = {
        (|| -> FileHash {
            let (data,): (&[u8],) = (&data,);
            return FileHash(hex::encode(Sha256::digest(data)));
        })()
    };
    let path = file_path(state, account, &hash);
    create_dirs(path.parent().unwrap()).await?;
    let tmp_path = path.with_extension(format!("tmp.{}", rand::random::<u64>()));
    tokio::fs::write(&tmp_path, &data)
        .await
        .context_with("Error writing file to disk", ea!(path = tmp_path.dbg_str()))?;
    tokio::fs::rename(&tmp_path, &path)
        .await
        .context_with("Error moving file into place", ea!(path = path.dbg_str()))?;
    let size = data.len() as i64;
    let created = DbTimeMs(Timestamp::now());
    let row = tx(&state.db, {
        let hash = hash.clone();
        move |db_tx| {
            good_ormning::sqlite::good_query!(
                db,
                r#"insert into file (account_id, hash, mime_type, size, created) values (${account_id_t = &account}, ${string = hash.0.as_str()}, ${string = mime_type.as_str()}, ${i64 = size}, ${time_ms_t = &created}) on conflict (account_id, hash) do update set created = excluded.created"#;
                db_tx
            ).map_err(|e| loga::err(e.0))?;
            return Ok(file_ref(db_tx, account, Some(hash.0))?.context("File row missing after insert")?);
        }
    }).await.context("Error recording file")?;
    return Ok(row);
}

pub fn file_ref(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    hash: Option<String>,
) -> Result<Option<FileRef>, loga::Error> {
    let Some(hash) = hash else {
        return Ok(None);
    };
    let row = good_ormning::sqlite::good_query_opt!(
        db,
        r#"select mime_type from file where account_id = ${account_id_t = &account} and hash = ${string = hash.as_str()}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?;
    return Ok(row.map(|mime_type| FileRef {
        hash: FileHash(hash),
        mime_type: mime_type,
    }));
}

pub async fn read_file(
    state: &State,
    account: DbAccountId,
    hash: &FileHash,
) -> Result<Option<(String, Vec<u8>)>, loga::Error> {
    let file_ref = tx(&state.db, {
        let hash = hash.clone();
        move |db_tx| {
            return file_ref(db_tx, account, Some(hash.0));
        }
    }).await.context("Error looking up file")?;
    let Some(file_ref) = file_ref else {
        return Ok(None);
    };
    let path = file_path(state, account, hash);
    let data =
        tokio::fs::read(&path).await.context_with("Error reading file from disk", ea!(path = path.dbg_str()))?;
    return Ok(Some((file_ref.mime_type, data)));
}

pub fn file_referenced(
    db_tx: &mut db::Db<Transaction<'_>>,
    account: DbAccountId,
    hash: &str,
) -> Result<bool, loga::Error> {
    if good_ormning::sqlite::good_query_opt!(
        db,
        r#"select 1 as x from identity where account_id = ${account_id_t = &account} and portrait = ${string = hash} and soft_deleted_at is null"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?.is_some() {
        return Ok(true);
    }
    if good_ormning::sqlite::good_query_opt!(
        db,
        r#"select 1 as x from channel where account_id = ${account_id_t = &account} and portrait = ${string = hash} and deleted is null"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?.is_some() {
        return Ok(true);
    }
    if good_ormning::sqlite::good_query_opt!(
        db,
        r#"select 1 as x from contact where account_id = ${account_id_t = &account} and portrait = ${string = hash}"#;
        db_tx
    ).map_err(|e| loga::err(e.0))?.is_some() {
        return Ok(true);
    }
    return Ok(false);
}

#[cfg(test)]
mod test {
    use {
        super::file_referenced,
        crate::{
            db,
            interface::db::DbAccountId,
        },
        rusqlite::Connection,
        shared::interface::shared::AccountId,
    };

    #[test]
    fn migrate_v2_rows_and_gc_references() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::migrate(&mut conn, Some(&|v| {
            let db::DbVersions::V2(db) = v else {
                return Ok(());
            };
            let c: &mut Connection = &mut *db.0;
            c.execute("insert into account (rowid, external_id) values (1, 'ext')", ()).unwrap();
            c
                .execute(
                    "insert into channel (account_id, identity, id, local_identity, idem, name, description, page_size, authenticated, public_read, public_post, backfill_done, next_sender_unique) values (1, 'owner', 'chan', 'me', 'idem', 'n', 'd', 20, 1, 0, 0, 0, 0)",
                    (),
                )
                .unwrap();
            c
                .execute(
                    "insert into file (account_id, hash, mime_type, size) values (1, 'h_ref', 'image/png', 3)",
                    (),
                )
                .unwrap();
            c
                .execute(
                    "insert into file (account_id, hash, mime_type, size) values (1, 'h_unref', 'image/png', 3)",
                    (),
                )
                .unwrap();
            c
                .execute(
                    "insert into contact (account_id, id, name, description, portrait) values (1, 'friend', 'n', 'd', 'h_ref')",
                    (),
                )
                .unwrap();
            return Ok(());
        })).unwrap();
        let (mode, secs): (String, Option<i64>) =
            conn
                .query_row(
                    "select retention_mode, retention_secs from channel",
                    (),
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
        assert_eq!(mode, "all");
        assert_eq!(secs, None);
        let created: i64 =
            conn.query_row("select created from file where hash = 'h_ref'", (), |r| r.get(0)).unwrap();
        assert_eq!(created, 0);
        let mut db_tx = db::Db(conn.transaction().unwrap());
        let account = DbAccountId(AccountId(1));
        assert!(file_referenced(&mut db_tx, account, "h_ref").unwrap());
        assert!(!file_referenced(&mut db_tx, account, "h_unref").unwrap());
    }
}
