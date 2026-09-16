use crate::response::{self, Response};
use crate::types::{data_dir, data_path};
use crate::{ensure_no_symlinks, path_metadata};
use hyper::http::StatusCode;
use std::path::{Path, PathBuf};
use tokio::fs;

const MAX_PATH_SIZE: usize = 4096;

pub(crate) type MvResult<T> = Result<T, (StatusCode, String)>;

pub async fn handle_mv(from: &str, to: &str) -> Response {
    match move_path(from, to).await {
        Ok(()) => response::status(StatusCode::OK),
        Err((status, message)) => response::text(status, message),
    }
}

pub(crate) async fn move_path(from: &str, to: &str) -> MvResult<()> {
    let from = mv_path(from, "source")?;
    let to = mv_path(to, "destination")?;
    let metadata = path_metadata(&from).await.map_err(move_error)?;
    ensure_no_symlinks(&to).await.map_err(move_error)?;
    let root = fs::canonicalize(data_dir()).await.map_err(move_error)?;
    let from = contained_path(from, &root).await?;
    let to = contained_path(to, &root).await?;

    if from == to {
        return Err((
            StatusCode::BAD_REQUEST,
            "source and destination must differ".to_string(),
        ));
    }

    match fs::symlink_metadata(to.as_path()).await {
        Ok(_) => {
            return Err((
                StatusCode::CONFLICT,
                "destination already exists".to_string(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(move_error(error)),
    }

    if metadata.is_dir() {
        let from = fs::canonicalize(from.as_path()).await.map_err(move_error)?;
        let parent = to.parent().ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "invalid destination path".to_string(),
            )
        })?;

        if parent.starts_with(&from) {
            return Err((
                StatusCode::BAD_REQUEST,
                "directory cannot be moved inside itself".to_string(),
            ));
        }
    }

    fs::rename(from, to).await.map_err(move_error)
}

fn mv_path(path: &str, name: &str) -> MvResult<PathBuf> {
    let path = path.trim();

    if path.is_empty() {
        return Err((StatusCode::BAD_REQUEST, format!("{name} path is required")));
    }
    if path.len() > MAX_PATH_SIZE || path.chars().any(|c| c.is_control() || matches!(c, '\\')) {
        return Err((StatusCode::BAD_REQUEST, format!("invalid {name} path")));
    }

    let path =
        data_path(path).ok_or_else(|| (StatusCode::BAD_REQUEST, format!("invalid {name} path")))?;

    if path.as_path() == data_dir() {
        return Err((
            StatusCode::FORBIDDEN,
            "data root cannot be moved or replaced".to_string(),
        ));
    }

    Ok(path)
}

async fn contained_path(path: PathBuf, root: &Path) -> MvResult<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))?;
    let parent = fs::canonicalize(parent).await.map_err(move_error)?;

    if !parent.starts_with(root) {
        return Err((
            StatusCode::FORBIDDEN,
            "path escapes data directory".to_string(),
        ));
    }

    let name = path
        .file_name()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))?;

    Ok(parent.join(name))
}

fn move_error(error: std::io::Error) -> (StatusCode, String) {
    match error.kind() {
        std::io::ErrorKind::NotFound => (StatusCode::NOT_FOUND, "path not found".to_string()),
        std::io::ErrorKind::AlreadyExists => (
            StatusCode::CONFLICT,
            "destination already exists".to_string(),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to move path: {error}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{mv_path, MAX_PATH_SIZE};
    use hyper::http::StatusCode;

    #[test]
    fn validates_mv_paths() {
        for path in ["file.txt", "/folder/file.txt", "árbol/東京.txt"] {
            assert!(mv_path(path, "source").is_ok());
        }

        assert_eq!(
            mv_path("", "source"),
            Err((
                StatusCode::BAD_REQUEST,
                "source path is required".to_string()
            ))
        );
        for path in ["/", "///", ".", "./", "/./"] {
            assert_eq!(
                mv_path(path, "destination"),
                Err((
                    StatusCode::FORBIDDEN,
                    "data root cannot be moved or replaced".to_string()
                ))
            );
        }

        for path in ["..", "../file", "folder/../file", "a\\b", "a\nb", "a\0b"] {
            assert_eq!(
                mv_path(path, "source"),
                Err((StatusCode::BAD_REQUEST, "invalid source path".to_string()))
            );
        }

        assert_eq!(
            mv_path(&"a".repeat(MAX_PATH_SIZE + 1), "destination"),
            Err((
                StatusCode::BAD_REQUEST,
                "invalid destination path".to_string()
            ))
        );
    }
}
