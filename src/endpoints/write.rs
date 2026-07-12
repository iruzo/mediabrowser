use crate::types::{data_dir, data_path};
use serde::Deserialize;
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use warp::http::StatusCode;
use warp::hyper::Body;
use warp::Reply;

const MAX_PATH_SIZE: usize = 4096;

pub(crate) type WriteResult<T> = Result<T, (StatusCode, String)>;

#[derive(Deserialize)]
pub struct WriteForm {
    path: String,
    content: String,
}

pub async fn handle_write(form: WriteForm) -> Result<warp::reply::Response, Infallible> {
    let response = match write_path(&form.path, form.content.as_bytes()).await {
        Ok(()) => warp::http::Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap(),
        Err((status, message)) => warp::reply::with_status(message, status).into_response(),
    };

    Ok(response)
}

pub(crate) async fn write_path(path: &str, content: &[u8]) -> WriteResult<()> {
    let path = file_path(path)?;
    fs::create_dir_all(data_dir()).await.map_err(write_error)?;
    let root = fs::canonicalize(data_dir()).await.map_err(write_error)?;
    let path = create_parent_dirs(path, &root).await?;

    match fs::symlink_metadata(&path).await {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => {
            return Err((
                StatusCode::CONFLICT,
                "path is not a regular file".to_string(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(write_error(error)),
    }

    write_file(&path, content).await
}

fn file_path(path: &str) -> WriteResult<PathBuf> {
    let path = path.trim();

    if path.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "path is required".to_string()));
    }
    if path.len() > MAX_PATH_SIZE || path.chars().any(|c| c.is_control() || matches!(c, '\\')) {
        return Err((StatusCode::BAD_REQUEST, "invalid path".to_string()));
    }

    let path =
        data_path(path).ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))?;

    if path.as_path() == data_dir() {
        return Err((
            StatusCode::FORBIDDEN,
            "data root cannot be written as a file".to_string(),
        ));
    }

    Ok(path)
}

async fn create_parent_dirs(path: PathBuf, root: &Path) -> WriteResult<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))?;
    let mut ancestor = parent.to_path_buf();

    loop {
        match fs::symlink_metadata(&ancestor).await {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if !ancestor.pop() {
                    return Err((StatusCode::BAD_REQUEST, "invalid path".to_string()));
                }
            }
            Err(error) => return Err(write_error(error)),
        }
    }

    let ancestor = fs::canonicalize(ancestor).await.map_err(write_error)?;
    if !ancestor.starts_with(root) {
        return Err((
            StatusCode::FORBIDDEN,
            "path escapes data directory".to_string(),
        ));
    }

    fs::create_dir_all(parent).await.map_err(write_error)?;
    let parent = fs::canonicalize(parent).await.map_err(write_error)?;

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

async fn write_file(path: &Path, content: &[u8]) -> WriteResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))?;
    let (mut file, temp_path) = create_temp_file(parent).await.map_err(write_error)?;

    let result = async {
        file.write_all(content).await?;
        file.flush().await
    }
    .await;

    drop(file);

    if let Err(error) = result {
        let _ = fs::remove_file(temp_path).await;
        return Err(write_error(error));
    }

    if let Err(error) = fs::rename(&temp_path, path).await {
        let _ = fs::remove_file(temp_path).await;
        return Err(write_error(error));
    }

    Ok(())
}

async fn create_temp_file(parent: &Path) -> std::io::Result<(tokio::fs::File, PathBuf)> {
    let mut id = 0_u64;

    loop {
        let path = parent.join(format!(".mediabrowser-write-{id}"));

        match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
        {
            Ok(file) => return Ok((file, path)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => id += 1,
            Err(error) => return Err(error),
        }
    }
}

fn write_error(error: std::io::Error) -> (StatusCode, String) {
    match error.kind() {
        std::io::ErrorKind::NotFound => (StatusCode::NOT_FOUND, "path not found".to_string()),
        std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::NotADirectory => (
            StatusCode::CONFLICT,
            "path conflicts with an existing file".to_string(),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to write file: {error}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{file_path, MAX_PATH_SIZE};
    use warp::http::StatusCode;

    #[test]
    fn validates_write_paths() {
        for path in ["file.txt", "/folder/file.txt", "árbol/東京.txt"] {
            assert!(file_path(path).is_ok());
        }

        for path in ["", " "] {
            assert_eq!(
                file_path(path),
                Err((StatusCode::BAD_REQUEST, "path is required".to_string()))
            );
        }

        for path in ["/", "///", ".", "./", "/./"] {
            assert_eq!(
                file_path(path),
                Err((
                    StatusCode::FORBIDDEN,
                    "data root cannot be written as a file".to_string()
                ))
            );
        }

        for path in ["..", "../file", "folder/../file", "a\\b", "a\nb", "a\0b"] {
            assert_eq!(
                file_path(path),
                Err((StatusCode::BAD_REQUEST, "invalid path".to_string()))
            );
        }

        assert_eq!(
            file_path(&"a".repeat(MAX_PATH_SIZE + 1)),
            Err((StatusCode::BAD_REQUEST, "invalid path".to_string()))
        );
    }
}
