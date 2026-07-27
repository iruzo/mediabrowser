use super::{grid_error, GridResult};
use crate::types::{data_dir, data_path};
use hyper::http::StatusCode;
use std::path::PathBuf;
use tokio::fs;

const MAX_PATH_SIZE: usize = 4096;

pub(super) async fn resolve_directory(path: &str) -> GridResult<PathBuf> {
    if path.len() > MAX_PATH_SIZE || path.chars().any(|c| c.is_control() || c == '\\') {
        return Err((StatusCode::BAD_REQUEST, "invalid path".to_string()));
    }

    let dir = data_path(path).ok_or((StatusCode::BAD_REQUEST, "invalid path".to_string()))?;
    let root = fs::canonicalize(data_dir()).await.map_err(grid_error)?;
    let dir = fs::canonicalize(dir).await.map_err(grid_error)?;

    if !dir.starts_with(&root) {
        return Err((
            StatusCode::FORBIDDEN,
            "path escapes data directory".to_string(),
        ));
    }
    if !fs::metadata(&dir).await.map_err(grid_error)?.is_dir() {
        return Err((
            StatusCode::BAD_REQUEST,
            "path is not a directory".to_string(),
        ));
    }

    Ok(dir)
}
