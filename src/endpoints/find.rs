use crate::types::{data_dir, data_path};
use serde::Deserialize;
use std::convert::Infallible;
use std::path::{Component, Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;
use warp::http::StatusCode;
use warp::Reply;

const MAX_PATH_SIZE: usize = 4096;
const MAX_QUERY_SIZE: usize = 4096;
const MAX_SEARCH_RESULTS: usize = 500;

pub(crate) type FindResult<T> = Result<T, (StatusCode, String)>;

#[derive(Deserialize)]
pub struct FindQuery {
    path: Option<String>,
    query: Option<String>,
}

pub async fn handle_find(query: FindQuery) -> Result<warp::reply::Response, Infallible> {
    let response = match find(query.path.as_deref(), query.query.as_deref()).await {
        Ok(paths) => warp::reply::json(&paths).into_response(),
        Err((status, message)) => warp::reply::with_status(message, status).into_response(),
    };

    Ok(response)
}

pub(crate) async fn find(path: Option<&str>, query: Option<&str>) -> FindResult<Vec<String>> {
    let (path, root) = resolve_directory(path).await?;
    let query = query.unwrap_or_default().trim();

    if query.len() > MAX_QUERY_SIZE {
        return Err((StatusCode::BAD_REQUEST, "query is too long".to_string()));
    }

    let terms: Vec<String> = query
        .split_whitespace()
        .map(|term| term.to_lowercase())
        .collect();
    let limit = if terms.is_empty() {
        None
    } else {
        Some(MAX_SEARCH_RESULTS)
    };

    tokio::task::spawn_blocking(move || search_paths(&path, &root, &terms, limit))
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to search directory: {error}"),
            )
        })?
}

async fn resolve_directory(path: Option<&str>) -> FindResult<(PathBuf, PathBuf)> {
    let path = directory_path(path)?;
    let root = fs::canonicalize(data_dir()).await.map_err(find_error)?;
    let path = if path.as_path() == data_dir() {
        root.clone()
    } else {
        fs::canonicalize(path).await.map_err(find_error)?
    };

    if !path.starts_with(&root) {
        return Err((
            StatusCode::FORBIDDEN,
            "path escapes data directory".to_string(),
        ));
    }

    let metadata = fs::metadata(&path).await.map_err(find_error)?;
    if !metadata.is_dir() {
        return Err((
            StatusCode::BAD_REQUEST,
            "path is not a directory".to_string(),
        ));
    }

    Ok((path, root))
}

fn directory_path(path: Option<&str>) -> FindResult<PathBuf> {
    let path = path.unwrap_or_default().trim();

    if path.len() > MAX_PATH_SIZE || path.chars().any(|c| c.is_control() || matches!(c, '\\')) {
        return Err((StatusCode::BAD_REQUEST, "invalid path".to_string()));
    }

    data_path(path).ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))
}

fn search_paths(
    path: &Path,
    root: &Path,
    terms: &[String],
    limit: Option<usize>,
) -> FindResult<Vec<String>> {
    let mut paths = Vec::new();

    for entry in WalkDir::new(path).min_depth(1).follow_links(false) {
        let entry = entry.map_err(walk_error)?;
        let file_type = entry.file_type();
        if !file_type.is_file() && !file_type.is_dir() {
            continue;
        }

        let Some(relative) = relative_path(path, entry.path(), false) else {
            continue;
        };
        if !path_matches(&relative, terms) {
            continue;
        }

        if let Some(path) = relative_path(root, entry.path(), file_type.is_dir()) {
            paths.push(path);
        }
        if limit == Some(paths.len()) {
            break;
        }
    }

    paths.sort_unstable();
    Ok(paths)
}

fn relative_path(root: &Path, path: &Path, is_dir: bool) -> Option<String> {
    let path = path.strip_prefix(root).ok()?;
    let mut value = String::new();

    for component in path.components() {
        let Component::Normal(component) = component else {
            return None;
        };
        let component = component.to_str()?;

        if !value.is_empty() {
            value.push('/');
        }
        value.push_str(component);
    }

    if value.is_empty() {
        return None;
    }
    if is_dir {
        value.push('/');
    }

    Some(value)
}

fn path_matches(path: &str, terms: &[String]) -> bool {
    let path = path.to_lowercase();
    terms.iter().all(|term| path.contains(term))
}

fn walk_error(error: walkdir::Error) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Failed to search directory: {error}"),
    )
}

fn find_error(error: std::io::Error) -> (StatusCode, String) {
    if error.kind() == std::io::ErrorKind::NotFound {
        (StatusCode::NOT_FOUND, "directory not found".to_string())
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to find paths: {error}"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{directory_path, path_matches, relative_path, MAX_PATH_SIZE};
    use std::path::Path;
    use warp::http::StatusCode;

    #[test]
    fn formats_file_and_directory_paths() {
        let root = Path::new("/data");

        assert_eq!(
            relative_path(root, Path::new("/data/folder"), true),
            Some("folder/".to_string())
        );
        assert_eq!(
            relative_path(root, Path::new("/data/folder/file.txt"), false),
            Some("folder/file.txt".to_string())
        );
    }

    #[test]
    fn matches_all_terms_without_case_sensitivity() {
        let terms = vec!["folder".to_string(), "report".to_string()];

        assert!(path_matches("Folder/Final Report.txt", &terms));
        assert!(!path_matches("Folder/Notes.txt", &terms));
        assert!(path_matches("Folder/Notes.txt", &[]));
    }

    #[test]
    fn validates_find_paths() {
        for path in [
            None,
            Some(""),
            Some("/"),
            Some("folder"),
            Some("árbol/東京"),
        ] {
            assert!(directory_path(path).is_ok());
        }

        for path in [Some(".."), Some("../folder"), Some("a\\b"), Some("a\nb")] {
            assert_eq!(
                directory_path(path),
                Err((StatusCode::BAD_REQUEST, "invalid path".to_string()))
            );
        }

        let path = "a".repeat(MAX_PATH_SIZE + 1);
        assert_eq!(
            directory_path(Some(&path)),
            Err((StatusCode::BAD_REQUEST, "invalid path".to_string()))
        );
    }
}
