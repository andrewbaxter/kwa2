use {
    loga::{
        ea,
        DebugDisplay,
        ResultContext,
    },
    std::path::Path,
    tokio::fs::create_dir_all,
};

pub async fn create_dirs(path: &Path) -> Result<(), loga::Error> {
    create_dir_all(path).await.context_with("Failed to create directory", ea!(path = path.dbg_str()))?;
    return Ok(());
}

pub async fn remove_file_if_exists(path: &Path) -> Result<(), loga::Error> {
    match tokio::fs::remove_file(path).await {
        Ok(_) => return Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(
            loga::err(e.to_string()).context_with("Error deleting file", ea!(path = path.dbg_str())),
        ),
    }
}
