use crate::mime::media_kind;
use crate::response::{self, Response};
use crate::types::{data_dir, data_path};
use crate::{path_metadata, walk};
use hyper::http::StatusCode;
use std::fmt::Write;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;
use tokio::fs;

const MAX_PATH_SIZE: usize = 4096;
const MAX_QUERY_SIZE: usize = 4096;
const MAX_SEARCH_RESULTS: usize = 500;

type FindResult<T> = Result<T, (StatusCode, String)>;

pub async fn handle_find(
    path: Option<&str>,
    query: Option<&str>,
    path_type: Option<&str>,
    recursive: Option<&str>,
    metadata: Option<&str>,
) -> Response {
    match find(path, query, path_type, recursive, metadata).await {
        Ok(json) => response::json(json),
        Err((status, message)) => response::text(status, message),
    }
}

async fn find(
    path: Option<&str>,
    query: Option<&str>,
    path_type: Option<&str>,
    recursive: Option<&str>,
    metadata: Option<&str>,
) -> FindResult<String> {
    let (path, root) = resolve_directory(path).await?;
    let query = query.unwrap_or_default().trim().to_string();
    let path_type = path_type.unwrap_or_default();

    if !matches!(path_type, "" | "all" | "dir" | "file") {
        return Err((StatusCode::BAD_REQUEST, "invalid type".to_string()));
    }
    let recursive = parse_recursive(recursive)?;
    let metadata = parse_metadata(metadata)?;

    let path_type = path_type.to_string();

    if query.len() > MAX_QUERY_SIZE {
        return Err((StatusCode::BAD_REQUEST, "query is too long".to_string()));
    }

    tokio::task::spawn_blocking(move || {
        let mut paths = if recursive {
            list_paths(&path, &root)?
        } else {
            list_direct_paths(&path, &root)?
        };
        filter_path_type(&mut paths, &path_type);
        filter_paths(&mut paths, &query);
        paths.sort_unstable();
        if !query.is_empty() {
            paths.truncate(MAX_SEARCH_RESULTS);
        }
        if metadata {
            Ok(json_metadata(&root, &paths))
        } else {
            Ok(json_paths(&paths))
        }
    })
    .await
    .map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to search directory: {error}"),
        )
    })?
}

fn filter_path_type(paths: &mut Vec<String>, path_type: &str) {
    if path_type == "dir" {
        paths.retain(|path| path.ends_with('/'));
    } else if path_type == "file" {
        paths.retain(|path| !path.ends_with('/'));
    }
}

fn parse_recursive(value: Option<&str>) -> FindResult<bool> {
    match value {
        None | Some("") | Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(_) => Err((StatusCode::BAD_REQUEST, "invalid recursive".to_string())),
    }
}

fn parse_metadata(value: Option<&str>) -> FindResult<bool> {
    match value {
        None | Some("") | Some("false") => Ok(false),
        Some("true") => Ok(true),
        Some(_) => Err((StatusCode::BAD_REQUEST, "invalid metadata".to_string())),
    }
}

async fn resolve_directory(path: Option<&str>) -> FindResult<(PathBuf, PathBuf)> {
    let path = directory_path(path)?;
    let metadata = path_metadata(&path).await.map_err(find_error)?;
    if !metadata.is_dir() {
        return Err((
            StatusCode::BAD_REQUEST,
            "path is not a directory".to_string(),
        ));
    }

    let root = fs::canonicalize(data_dir()).await.map_err(find_error)?;
    let path = fs::canonicalize(path).await.map_err(find_error)?;

    if !path.starts_with(&root) {
        return Err((
            StatusCode::FORBIDDEN,
            "path escapes data directory".to_string(),
        ));
    }

    Ok((path, root))
}

fn directory_path(path: Option<&str>) -> FindResult<PathBuf> {
    let path = path.unwrap_or_default();

    if path.len() > MAX_PATH_SIZE || path.chars().any(|c| c.is_control() || c == '\\') {
        return Err((StatusCode::BAD_REQUEST, "invalid path".to_string()));
    }

    data_path(path).ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))
}

fn list_paths(path: &Path, root: &Path) -> FindResult<Vec<String>> {
    let mut paths = Vec::new();

    for entry in walk(path).map_err(walk_error)? {
        if !entry.file_type.is_file() && !entry.file_type.is_dir() {
            continue;
        }

        if let Some(path) = relative_path(root, &entry.path, entry.file_type.is_dir()) {
            paths.push(path);
        }
    }

    Ok(paths)
}

fn list_direct_paths(path: &Path, root: &Path) -> FindResult<Vec<String>> {
    let mut paths = Vec::new();
    let entries = std::fs::read_dir(path).map_err(walk_error)?;

    for entry in entries {
        let entry = entry.map_err(walk_error)?;
        let file_type = {
            #[cfg(windows)]
            {
                std::fs::symlink_metadata(entry.path())
                    .map_err(walk_error)?
                    .file_type()
            }
            #[cfg(not(windows))]
            {
                entry.file_type().map_err(walk_error)?
            }
        };
        if file_type.is_symlink() || (!file_type.is_file() && !file_type.is_dir()) {
            continue;
        }

        if let Some(path) = relative_path(root, &entry.path(), file_type.is_dir()) {
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
        push_json_string(&mut json, path);
    }

    json.push(']');
    json
}

fn json_metadata(root: &Path, paths: &[String]) -> String {
    let mut json = String::with_capacity(paths.len() * 64 + 2);
    json.push('[');

    let mut first = true;
    for path in paths {
        let relative = path.strip_suffix('/').unwrap_or(path);
        let metadata = std::fs::symlink_metadata(root.join(relative)).ok();
        if metadata
            .as_ref()
            .map(|value| value.file_type().is_symlink())
            .unwrap_or(false)
        {
            continue;
        }

        if !first {
            json.push(',');
        }
        first = false;

        let size = metadata.as_ref().map_or(0, std::fs::Metadata::len);
        let date = metadata
            .and_then(|value| value.modified().ok())
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |value| value.as_secs());
        let kind = if path.ends_with('/') {
            "text"
        } else {
            media_kind(Path::new(path))
        };

        json.push_str("{\"path\":");
        push_json_string(&mut json, path);
        let _ = write!(
            json,
            ",\"size\":{size},\"date\":{date},\"kind\":\"{kind}\"}}"
        );
    }

    json.push(']');
    json
}

fn push_json_string(json: &mut String, value: &str) {
    json.push('"');

    for character in value.chars() {
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

fn walk_error(error: std::io::Error) -> (StatusCode, String) {
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
    use super::{
        directory_path, filter_path_type, filter_paths, json_metadata, json_paths,
        list_direct_paths, parse_metadata, parse_recursive, relative_path, MAX_PATH_SIZE,
    };
    use hyper::http::StatusCode;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn encodes_path_metadata_as_json() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("mediabrowser-metadata-{suffix}"));
        let name = "a\"b.txt";
        std::fs::create_dir(&root).expect("create test directory");
        std::fs::write(root.join(name), b"test").expect("create test file");
        let date = std::fs::metadata(root.join(name))
            .expect("read test metadata")
            .modified()
            .expect("read modified time")
            .duration_since(UNIX_EPOCH)
            .expect("modified time before epoch")
            .as_secs();

        assert_eq!(
            json_metadata(
                &root,
                &[
                    name.to_string(),
                    "missing.WMV".to_string(),
                    "movie.mp4/".to_string(),
                ]
            ),
            format!(
                "[{{\"path\":\"a\\\"b.txt\",\"size\":4,\"date\":{date},\"kind\":\"text\"}},{{\"path\":\"missing.WMV\",\"size\":0,\"date\":0,\"kind\":\"video\"}},{{\"path\":\"movie.mp4/\",\"size\":0,\"date\":0,\"kind\":\"text\"}}]"
            )
        );
        std::fs::remove_dir_all(root).expect("remove test directory");
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
    fn filters_path_types_before_query_terms() {
        let paths = vec![
            "photos/".to_string(),
            "photos/trip/".to_string(),
            "photos/trip/image.jpg".to_string(),
            "notes.txt".to_string(),
        ];

        let mut dirs = paths.clone();
        filter_path_type(&mut dirs, "dir");
        filter_paths(&mut dirs, "trip");
        assert_eq!(dirs, ["photos/trip/"]);

        let mut files = paths.clone();
        filter_path_type(&mut files, "file");
        filter_paths(&mut files, "trip");
        assert_eq!(files, ["photos/trip/image.jpg"]);

        let mut all = paths.clone();
        filter_path_type(&mut all, "all");
        assert_eq!(all, paths);

        filter_path_type(&mut all, "");
        assert_eq!(all, paths);
    }

    #[test]
    fn parses_recursive_mode() {
        assert_eq!(parse_recursive(None), Ok(true));
        assert_eq!(parse_recursive(Some("")), Ok(true));
        assert_eq!(parse_recursive(Some("true")), Ok(true));
        assert_eq!(parse_recursive(Some("false")), Ok(false));
        assert_eq!(
            parse_recursive(Some("other")),
            Err((StatusCode::BAD_REQUEST, "invalid recursive".to_string()))
        );
    }

    #[test]
    fn parses_metadata_mode() {
        assert_eq!(parse_metadata(None), Ok(false));
        assert_eq!(parse_metadata(Some("")), Ok(false));
        assert_eq!(parse_metadata(Some("false")), Ok(false));
        assert_eq!(parse_metadata(Some("true")), Ok(true));
        assert_eq!(
            parse_metadata(Some("other")),
            Err((StatusCode::BAD_REQUEST, "invalid metadata".to_string()))
        );
    }

    #[test]
    fn lists_only_direct_paths_when_recursion_is_disabled() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("mediabrowser-find-{suffix}"));
        std::fs::create_dir_all(root.join("child/nested")).expect("create test directories");
        std::fs::write(root.join("root.txt"), []).expect("create root file");
        std::fs::write(root.join("child/nested.txt"), []).expect("create nested file");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("root.txt", root.join("file-link"))
                .expect("create file link");
            std::os::unix::fs::symlink("child", root.join("directory-link"))
                .expect("create directory link");
        }

        let mut paths = list_direct_paths(&root, &root).expect("list direct paths");
        paths.sort_unstable();

        assert_eq!(paths, ["child/", "root.txt"]);
        std::fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn validates_find_paths() {
        for path in [
            None,
            Some(""),
            Some("/"),
            Some("folder"),
            Some(" folder "),
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
