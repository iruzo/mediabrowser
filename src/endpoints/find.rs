use crate::response::{self, Response};
use crate::types::{data_dir, data_path};
use hyper::http::StatusCode;
use serde::Deserialize;
use std::fmt::Write;
use std::path::{Component, Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

const MAX_PATH_SIZE: usize = 4096;
const MAX_QUERY_SIZE: usize = 4096;
const MAX_SEARCH_RESULTS: usize = 500;

type FindResult<T> = Result<T, (StatusCode, String)>;

#[derive(Deserialize)]
pub struct FindQuery {
    path: Option<String>,
    query: Option<String>,
}

pub async fn handle_find(query: FindQuery) -> Response {
    match find(query.path.as_deref(), query.query.as_deref()).await {
        Ok(paths) => response::json(json_paths(&paths)),
        Err((status, message)) => response::text(status, message),
    }
}

async fn find(path: Option<&str>, query: Option<&str>) -> FindResult<Vec<String>> {
    let (path, root) = resolve_directory(path).await?;
    let query = query.unwrap_or_default().trim().to_string();

    if query.len() > MAX_QUERY_SIZE {
        return Err((StatusCode::BAD_REQUEST, "query is too long".to_string()));
    }

    tokio::task::spawn_blocking(move || {
        let mut paths = list_paths(&path, &root)?;
        filter_paths(&mut paths, &query);
        paths.sort_unstable();
        if !query.is_empty() {
            paths.truncate(MAX_SEARCH_RESULTS);
        }
        Ok(paths)
    })
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
    let path = fs::canonicalize(path).await.map_err(find_error)?;

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

    if path.len() > MAX_PATH_SIZE || path.chars().any(|c| c.is_control() || c == '\\') {
        return Err((StatusCode::BAD_REQUEST, "invalid path".to_string()));
    }

    data_path(path).ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))
}

fn list_paths(path: &Path, root: &Path) -> FindResult<Vec<String>> {
    let mut paths = Vec::new();

    for entry in WalkDir::new(path).min_depth(1).follow_links(false) {
        let entry = entry.map_err(walk_error)?;
        let file_type = entry.file_type();
        if !file_type.is_file() && !file_type.is_dir() {
            continue;
        }

        if let Some(path) = relative_path(root, entry.path(), file_type.is_dir()) {
            paths.push(path);
        }
    }

    Ok(paths)
}

fn filter_paths(paths: &mut Vec<String>, query: &str) {
    let case_sensitive = query.chars().any(char::is_uppercase);

    for term in query.split_whitespace() {
        if case_sensitive {
            paths.retain(|path| path.contains(term));
        } else {
            let term = term.to_lowercase();
            paths.retain(|path| path.to_lowercase().contains(&term));
        }
    }
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

fn json_paths(paths: &[String]) -> String {
    let mut json = String::with_capacity(paths.len() * 16 + 2);
    json.push('[');

    for (index, path) in paths.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('"');

        for character in path.chars() {
            match character {
                '"' => json.push_str("\\\""),
                '\\' => json.push_str("\\\\"),
                character if (character as u32) < 0x20 => {
                    let _ = write!(json, "\\u{:04x}", character as u32);
                }
                character => json.push(character),
            }
        }

        json.push('"');
    }

    json.push(']');
    json
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
    use super::{directory_path, filter_paths, json_paths, relative_path, MAX_PATH_SIZE};
    use hyper::http::StatusCode;
    use std::path::Path;

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
    fn encodes_paths_as_json_strings() {
        assert_eq!(json_paths(&[]), "[]");
        assert_eq!(
            json_paths(&["folder/".to_string(), "a\"b\\c\n東京.txt".to_string()]),
            "[\"folder/\",\"a\\\"b\\\\c\\u000a東京.txt\"]"
        );
    }

    #[test]
    fn filters_terms_with_smart_case() {
        let mut paths = vec![
            "Folder/Final Report.txt".to_string(),
            "Folder/Notes.txt".to_string(),
        ];
        filter_paths(&mut paths, "folder report");
        assert_eq!(paths, ["Folder/Final Report.txt"]);

        let mut paths = vec![
            "Folder/Final Report.txt".to_string(),
            "folder/final report.txt".to_string(),
        ];
        filter_paths(&mut paths, "Report");
        assert_eq!(paths, ["Folder/Final Report.txt"]);

        let mut paths = vec!["Folder/Notes.txt".to_string()];
        filter_paths(&mut paths, "");
        assert_eq!(paths, ["Folder/Notes.txt"]);
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
