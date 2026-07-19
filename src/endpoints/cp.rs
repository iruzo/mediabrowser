use crate::response::{self, Response};
use crate::types::{data_dir, data_path};
use hyper::http::StatusCode;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

const MAX_PATH_SIZE: usize = 4096;

pub(crate) type CpResult<T> = Result<T, (StatusCode, String)>;

#[derive(Deserialize)]
pub struct CpForm {
    from: String,
    to: String,
}

pub async fn handle_cp(form: CpForm) -> Response {
    match copy_path(&form.from, &form.to).await {
        Ok(()) => response::status(StatusCode::OK),
        Err((status, message)) => response::text(status, message),
    }
}

pub(crate) async fn copy_path(from: &str, to: &str) -> CpResult<()> {
    let from = cp_path(from, "source")?;
    let to = cp_path(to, "destination")?;
    let root = fs::canonicalize(data_dir()).await.map_err(copy_error)?;
    let from = contained_path(from, &root).await?;
    let to = contained_path(to, &root).await?;

    if from == to {
        return Err((
            StatusCode::BAD_REQUEST,
            "source and destination must differ".to_string(),
        ));
    }

    let metadata = fs::symlink_metadata(&from).await.map_err(copy_error)?;

    match fs::symlink_metadata(&to).await {
        Ok(_) => {
            return Err((
                StatusCode::CONFLICT,
                "destination already exists".to_string(),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(copy_error(error)),
    }

    if metadata.is_dir() {
        let from = fs::canonicalize(&from).await.map_err(copy_error)?;

        if to.starts_with(&from) {
            return Err((
                StatusCode::BAD_REQUEST,
                "directory cannot be copied inside itself".to_string(),
            ));
        }

        return tokio::task::spawn_blocking(move || copy_directory(&from, &to))
            .await
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to copy path: {error}"),
                )
            })?;
    }

    if !metadata.is_file() {
        return Err((
            StatusCode::BAD_REQUEST,
            "source is not a file or directory".to_string(),
        ));
    }

    fs::copy(from, to).await.map(|_| ()).map_err(copy_error)
}

fn copy_directory(from: &Path, to: &Path) -> CpResult<()> {
    for entry in WalkDir::new(from).follow_links(false) {
        let entry = entry.map_err(walk_error)?;
        let target = match entry.path().strip_prefix(from) {
            Ok(relative) => to.join(relative),
            Err(_) => continue,
        };
        let file_type = entry.file_type();

        if file_type.is_dir() {
            std::fs::create_dir_all(&target).map_err(copy_error)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), &target)
                .map(|_| ())
                .map_err(copy_error)?;
        }
    }

    Ok(())
}

fn cp_path(path: &str, name: &str) -> CpResult<PathBuf> {
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
            "data root cannot be copied or replaced".to_string(),
        ));
    }

    Ok(path)
}

async fn contained_path(path: PathBuf, root: &Path) -> CpResult<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))?;
    let parent = fs::canonicalize(parent).await.map_err(copy_error)?;

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

fn copy_error(error: std::io::Error) -> (StatusCode, String) {
    match error.kind() {
        std::io::ErrorKind::NotFound => (StatusCode::NOT_FOUND, "path not found".to_string()),
        std::io::ErrorKind::AlreadyExists => (
            StatusCode::CONFLICT,
            "destination already exists".to_string(),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to copy path: {error}"),
        ),
    }
}

fn walk_error(error: walkdir::Error) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Failed to copy path: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::{cp_path, MAX_PATH_SIZE};
    use hyper::http::StatusCode;

    #[test]
    fn validates_cp_paths() {
        for path in ["file.txt", "/folder/file.txt", "árbol/東京.txt"] {
            assert!(cp_path(path, "source").is_ok());
        }

        assert_eq!(
            cp_path("", "source"),
            Err((
                StatusCode::BAD_REQUEST,
                "source path is required".to_string()
            ))
        );
        for path in ["/", "///", ".", "./", "/./"] {
            assert_eq!(
                cp_path(path, "destination"),
                Err((
                    StatusCode::FORBIDDEN,
                    "data root cannot be copied or replaced".to_string()
                ))
            );
        }

        for path in ["..", "../file", "folder/../file", "a\\b", "a\nb", "a\0b"] {
            assert_eq!(
                cp_path(path, "source"),
                Err((StatusCode::BAD_REQUEST, "invalid source path".to_string()))
            );
        }

        assert_eq!(
            cp_path(&"a".repeat(MAX_PATH_SIZE + 1), "destination"),
            Err((
                StatusCode::BAD_REQUEST,
                "invalid destination path".to_string()
            ))
        );
    }
}
