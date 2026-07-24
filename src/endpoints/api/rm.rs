use crate::response::{self, Response};
use crate::types::{data_dir, data_path};
use hyper::http::StatusCode;
use std::path::PathBuf;
use tokio::fs;

const MAX_PATH_SIZE: usize = 4096;

pub(crate) type RmResult<T> = Result<T, (StatusCode, String)>;

pub async fn handle_rm(path: &str) -> Response {
    match remove_path(path).await {
        Ok(()) => response::status(StatusCode::OK),
        Err((status, message)) => response::text(status, message),
    }
}

pub(crate) async fn remove_path(path: &str) -> RmResult<()> {
    let path = rm_path(path).map_err(|(status, message)| (status, message.to_string()))?;
    let path = contained_path(path).await?;
    let metadata = fs::symlink_metadata(&path).await.map_err(remove_error)?;

    if metadata.is_dir() {
        fs::remove_dir_all(path).await.map_err(remove_error)
    } else {
        fs::remove_file(path).await.map_err(remove_error)
    }
}

fn rm_path(path: &str) -> Result<PathBuf, (StatusCode, &'static str)> {
    let path = path.trim();

    if path.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "path is required"));
    }
    if path.len() > MAX_PATH_SIZE || path.chars().any(|c| c.is_control() || matches!(c, '\\')) {
        return Err((StatusCode::BAD_REQUEST, "invalid path"));
    }

    let path = data_path(path).ok_or((StatusCode::BAD_REQUEST, "invalid path"))?;
    if path.as_path() == data_dir() {
        return Err((StatusCode::FORBIDDEN, "data root cannot be removed"));
    }

    Ok(path)
}

async fn contained_path(path: PathBuf) -> RmResult<PathBuf> {
    let root = fs::canonicalize(data_dir()).await.map_err(remove_error)?;
    let parent = path
        .parent()
        .ok_or((StatusCode::BAD_REQUEST, "invalid path".to_string()))?;
    let parent = fs::canonicalize(parent).await.map_err(remove_error)?;

    if !parent.starts_with(&root) {
        return Err((
            StatusCode::FORBIDDEN,
            "path escapes data directory".to_string(),
        ));
    }

    let name = path
        .file_name()
        .ok_or((StatusCode::BAD_REQUEST, "invalid path".to_string()))?;

    Ok(parent.join(name))
}

fn remove_error(error: std::io::Error) -> (StatusCode, String) {
    if error.kind() == std::io::ErrorKind::NotFound {
        (StatusCode::NOT_FOUND, "path not found".to_string())
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to remove path: {error}"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{rm_path, MAX_PATH_SIZE};
    use hyper::http::StatusCode;

    #[test]
    fn validates_rm_paths() {
        for path in ["file.txt", "/folder/file.txt", "árbol/東京.txt"] {
            assert!(rm_path(path).is_ok());
        }

        for path in ["", " "] {
            assert_eq!(
                rm_path(path),
                Err((StatusCode::BAD_REQUEST, "path is required"))
            );
        }

        for path in ["/", "///", ".", "./", "/./"] {
            assert_eq!(
                rm_path(path),
                Err((StatusCode::FORBIDDEN, "data root cannot be removed"))
            );
        }

        for path in ["..", "../file", "folder/../file", "a\\b", "a\nb", "a\0b"] {
            assert_eq!(
                rm_path(path),
                Err((StatusCode::BAD_REQUEST, "invalid path"))
            );
        }

        assert_eq!(
            rm_path(&"a".repeat(MAX_PATH_SIZE + 1)),
            Err((StatusCode::BAD_REQUEST, "invalid path"))
        );
    }
}
