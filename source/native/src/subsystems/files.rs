use {
    crate::{
        db,
        dbutil::tx,
        fsutil::create_dirs,
        interface::db::DbAccountId,
        State,
    },
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

pub fn hash_bytes(data: &[u8]) -> FileHash {
    return FileHash(hex::encode(Sha256::digest(data)));
}

fn file_path(state: &State, account: DbAccountId, hash: &FileHash) -> PathBuf {
    return state.files_dir.join(account.0.0.to_string()).join(&hash.0);
}

pub async fn store_file(
    state: &State,
    account: DbAccountId,
    mime_type: String,
    data: Vec<u8>,
) -> Result<FileRef, loga::Error> {
    let hash = hash_bytes(&data);
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
    let row = tx(&state.db, {
        let hash = hash.clone();
        move |db_tx| {
            good_ormning::sqlite::good_query!(
                db,
                r#"insert into file (account_id, hash, mime_type, size) values (${account_id_t = &account}, ${string = hash.0.as_str()}, ${string = mime_type.as_str()}, ${i64 = size}) on conflict (account_id, hash) do nothing"#;
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
    let row =
        good_ormning::sqlite::good_query_opt!(
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
