mod render;
mod resolve;

use crate::response::{self, Response};
use crate::types::{data_dir, data_path};
use hyper::http::StatusCode;
use render::{render_grid, sort_items, GridItem, Sort};
use resolve::resolve_directory;
use tokio::fs;

const MAX_ITEMS: usize = 500;
const MAX_PATH_SIZE: usize = 4096;

type GridResult<T> = Result<T, (StatusCode, String)>;

pub async fn handle_grid(path: Option<&str>) -> Response {
    let path = path.unwrap_or_default();

    match grid(path).await {
        Ok(html) => response::html(html),
        Err((status, message)) => response::text(status, message),
    }
}

pub async fn handle_grid_paths(paths: Vec<String>) -> Response {
    match grid_paths(paths).await {
        Ok(html) => response::html(html),
        Err((status, message)) => response::text(status, message),
    }
}

async fn grid(path: &str) -> GridResult<String> {
    let dir = resolve_directory(path).await?;

    let mut entries = fs::read_dir(&dir).await.map_err(grid_error)?;
    let mut items = Vec::new();

    while let Some(entry) = entries.next_entry().await.map_err(grid_error)? {
        let Ok(file_type) = entry.file_type().await else {
            continue;
        };
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };

        if file_type.is_dir() || file_type.is_file() {
            let metadata = entry.metadata().await.ok();
            items.push(GridItem::new(
                name,
                file_type.is_dir(),
                metadata.as_ref().map_or(0, |metadata| metadata.len()),
                metadata.and_then(|metadata| metadata.modified().ok()),
            ));
        }
    }

    sort_items(&mut items, Sort::parse("name"));

    Ok(render_grid(path, &items))
}

async fn grid_paths(paths: Vec<String>) -> GridResult<String> {
    if paths.len() > MAX_ITEMS {
        return Err((StatusCode::BAD_REQUEST, "too many paths".to_string()));
    }

    let root = fs::canonicalize(data_dir()).await.map_err(grid_error)?;
    let mut items = Vec::with_capacity(paths.len());

    for path in paths {
        let path = path.trim_matches('/');

        if path.is_empty()
            || path.len() > MAX_PATH_SIZE
            || path.chars().any(|c| c.is_control() || c == '\\')
        {
            return Err((StatusCode::BAD_REQUEST, "invalid path".to_string()));
        }

        let file =
            data_path(path).ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))?;
        let file = fs::canonicalize(file).await.map_err(grid_error)?;

        if !file.starts_with(&root) {
            return Err((
                StatusCode::FORBIDDEN,
                "path escapes data directory".to_string(),
            ));
        }

        let metadata = fs::metadata(file).await.map_err(grid_error)?;
        if metadata.is_dir() || metadata.is_file() {
            let modified = metadata.modified().ok();
            items.push(GridItem::new(
                path.to_string(),
                metadata.is_dir(),
                metadata.len(),
                modified,
            ));
        }
    }

    Ok(render_grid("", &items))
}

fn grid_error(error: std::io::Error) -> (StatusCode, String) {
    if error.kind() == std::io::ErrorKind::NotFound {
        (StatusCode::NOT_FOUND, "directory not found".to_string())
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to list directory: {error}"),
        )
    }
}
