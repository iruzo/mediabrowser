use crate::response::{self, Response};
use crate::types::data_path;
use crate::{path_metadata, serve_file};
use hyper::http::{HeaderMap, StatusCode};
use std::path::PathBuf;

const MAX_PATH_SIZE: usize = 4096;

pub async fn handle_cat(path: Option<&str>, headers: &HeaderMap) -> Response {
    let Some(path) = cat_path(path) else {
        return not_found();
    };
    let Ok(metadata) = path_metadata(&path).await else {
        return not_found();
    };

    if !metadata.is_file() {
        return not_found();
    }

    serve_file(&path, headers, metadata.len()).await
}

fn cat_path(path: Option<&str>) -> Option<PathBuf> {
    let path = path?;

    if path.len() > MAX_PATH_SIZE || path.chars().any(|c| c.is_control() || c == '\\') {
        return None;
    }

    data_path(path)
}

fn not_found() -> Response {
    response::text(StatusCode::NOT_FOUND, "Not found")
}

#[cfg(test)]
mod tests {
    use super::{cat_path, MAX_PATH_SIZE};

    #[test]
    fn validates_cat_paths() {
        for path in [
            Some("file.txt"),
            Some("/folder/file.txt"),
            Some("file name.txt"),
            Some("árbol/東京.txt"),
        ] {
            assert!(cat_path(path).is_some());
        }

        for path in [
            None,
            Some(".."),
            Some("../file"),
            Some("folder/../file"),
            Some("a\\b"),
            Some("a\nb"),
        ] {
            assert!(cat_path(path).is_none());
        }

        let path = "a".repeat(MAX_PATH_SIZE + 1);
        assert!(cat_path(Some(&path)).is_none());
    }
}
