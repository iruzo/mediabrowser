use crate::types::{data_dir, data_path};
use serde::Deserialize;
use std::convert::Infallible;
use std::path::PathBuf;
use tokio::fs;
use warp::http::StatusCode;
use warp::hyper::Body;
use warp::Reply;

type MkdirResult<T> = Result<T, (StatusCode, String)>;

#[derive(Deserialize)]
pub struct MkdirForm {
    path: String,
}

pub async fn handle_mkdir(form: MkdirForm) -> Result<warp::reply::Response, Infallible> {
    let response = match create_dirs(&form.path).await {
        Ok(()) => warp::http::Response::builder()
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap(),
        Err((status, message)) => warp::reply::with_status(message, status).into_response(),
    };

    Ok(response)
}

pub(crate) async fn create_dirs(path: &str) -> MkdirResult<()> {
    let path =
        folder_path(path).map_err(|message| (StatusCode::BAD_REQUEST, message.to_string()))?;

    fs::create_dir_all(path).await.map_err(mkdir_error)
}

fn mkdir_error(error: std::io::Error) -> (StatusCode, String) {
    if matches!(
        error.kind(),
        std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::NotADirectory
    ) {
        (
            StatusCode::CONFLICT,
            "folder path conflicts with an existing file".to_string(),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create folder: {error}"),
        )
    }
}

fn folder_path(path: &str) -> Result<PathBuf, &'static str> {
    let path = path.trim();

    if path.is_empty() {
        return Err("folder path is required");
    }
    if path.chars().any(|c| c.is_control() || matches!(c, '\\')) {
        return Err("invalid folder path");
    }

    let path = data_path(path).ok_or("invalid folder path")?;
    if path.as_path() == data_dir() {
        return Err("folder path is required");
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::folder_path;

    #[test]
    fn validates_folder_paths() {
        for path in ["folder", "/folder/subfolder", "árbol/東京", "./folder"] {
            assert!(folder_path(path).is_ok());
        }

        for path in ["", " ", "/", "///", ".", "./", "/./"] {
            assert_eq!(folder_path(path), Err("folder path is required"));
        }

        for path in ["..", "../folder", "folder/../other", "a\\b", "a\nb", "a\0b"] {
            assert_eq!(folder_path(path), Err("invalid folder path"));
        }
    }
}
