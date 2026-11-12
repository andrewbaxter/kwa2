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
        mime_is_media,
        FileHash,
        FileRef,
    },
    std::{
        path::{
            Path,
            PathBuf,
        },
        pin::Pin,
        task::{
            Context,
            Poll,
        },
    },
    tokio::io::AsyncWrite,
};

pub const THUMBNAIL_MIME_TYPE: &str = "image/jpeg";

pub fn file_path(state: &State, account: DbAccountId, hash: &FileHash) -> PathBuf {
    return account_files_dir(state, account).join(&hash.0);
}

pub fn thumbnail_path(path: &Path) -> PathBuf {
    return path.with_extension("thumb");
}

pub fn tmp_path_in(dir: &Path) -> PathBuf {
    return dir.join(format!("tmp.{}", rand::random::<u64>()));
}

pub fn file_cache_path(
    state: &State,
    owner: spaghettinuum::interface::identity::Identity,
    hash: &str,
    thumbnail: bool,
) -> PathBuf {
    let path = state.file_cache_dir.join(owner.to_string()).join(hash);
    if thumbnail {
        return thumbnail_path(&path);
    }
    return path;
}

pub fn account_files_dir(state: &State, account: DbAccountId) -> PathBuf {
    return state.files_dir.join(account.0.0.to_string());
}

pub struct HashingWriter<W> {
    pub inner: W,
    pub hasher: Sha256,
    pub size: u64,
    pub limit: u64,
}

impl<W: AsyncWrite + Unpin> AsyncWrite for HashingWriter<W> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        if self.size + buf.len() as u64 > self.limit {
            return Poll::Ready(
                Err(std::io::Error::other(format!("File exceeds size limit of {} bytes", self.limit))),
            );
        }
        let res = Pin::new(&mut self.inner).poll_write(cx, buf);
        if let Poll::Ready(Ok(written)) = &res {
            let written = *written;
            self.hasher.update(&buf[..written]);
            self.size += written as u64;
        }
        return res;
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        return Pin::new(&mut self.inner).poll_flush(cx);
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        return Pin::new(&mut self.inner).poll_shutdown(cx);
    }
}

pub fn finish_hash(hasher: Sha256) -> FileHash {
    return FileHash(hex::encode(hasher.finalize()));
}

pub async fn store_file_tmp(
    state: &State,
    account: DbAccountId,
    mime_type: String,
    tmp_path: &Path,
    hash: FileHash,
    size: u64,
) -> Result<FileRef, loga::Error> {
    let path = file_path(state, account, &hash);
    tokio::fs::rename(tmp_path, &path)
        .await
        .context_with("Error moving file into place", ea!(from = tmp_path.dbg_str(), to = path.dbg_str()))?;
    if mime_is_media(&mime_type) {
        let thumb_path = thumbnail_path(&path);
        if !tokio::fs::try_exists(&thumb_path)
            .await
            .context_with("Error checking for existing thumbnail", ea!(path = thumb_path.dbg_str()))? {
            let mut thumb_tmp_path = tmp_path_in(path.parent().unwrap()).into_os_string();
            thumb_tmp_path.push(".jpg");
            let thumb_tmp_path = PathBuf::from(thumb_tmp_path);
            let output =
                tokio::process::Command::new(&state.ffmpeg)
                    .arg("-nostdin")
                    .arg("-loglevel")
                    .arg("error")
                    .arg("-i")
                    .arg(&path)
                    .arg("-vf")
                    .arg("thumbnail,scale='min(512,iw)':'min(512,ih)':force_original_aspect_ratio=decrease")
                    .arg("-frames:v")
                    .arg("1")
                    .arg("-f")
                    .arg("image2")
                    .arg("-c:v")
                    .arg("mjpeg")
                    .arg("-q:v")
                    .arg("4")
                    .arg(&thumb_tmp_path)
                    .output()
                    .await
                    .context_with("Error running ffmpeg", ea!(ffmpeg = state.ffmpeg.dbg_str()))?;
            if output.status.success() {
                tokio::fs::rename(&thumb_tmp_path, &thumb_path)
                    .await
                    .context_with("Error moving thumbnail into place", ea!(path = thumb_path.dbg_str()))?;
            } else {
                state
                    .log
                    .log_with(
                        loga::WARN,
                        "Thumbnail generation failed",
                        ea!(
                            path = path.dbg_str(),
                            mime_type = mime_type,
                            status = output.status,
                            stderr = String::from_utf8_lossy(&output.stderr)
                        ),
                    );
                crate::fsutil::remove_file_if_exists(&thumb_tmp_path).await?;
            }
        }
    }
    let size = i64::try_from(size).context("File size out of range")?;
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

pub async fn store_file(
    state: &State,
    account: DbAccountId,
    mime_type: String,
    data: Vec<u8>,
) -> Result<FileRef, loga::Error> {
    let hash = FileHash(hex::encode(Sha256::digest(&data)));
    let dir = account_files_dir(state, account);
    create_dirs(&dir).await?;
    let tmp_path = tmp_path_in(&dir);
    tokio::fs::write(&tmp_path, &data)
        .await
        .context_with("Error writing file to disk", ea!(path = tmp_path.dbg_str()))?;
    return Ok(store_file_tmp(state, account, mime_type, &tmp_path, hash, data.len() as u64).await?);
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

pub async fn file_local(
    state: &State,
    account: DbAccountId,
    hash: &FileHash,
    thumbnail: bool,
) -> Result<Option<(String, PathBuf)>, loga::Error> {
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
    if thumbnail {
        return Ok(Some((THUMBNAIL_MIME_TYPE.to_string(), thumbnail_path(&path))));
    }
    return Ok(Some((file_ref.mime_type, path)));
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
    if good_ormning::sqlite::good_query_opt!(
        db,
        r#"select 1 as x from message_file where account_id = ${account_id_t = &account} and hash = ${string = hash} limit ${i64 = 1}"#;
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
