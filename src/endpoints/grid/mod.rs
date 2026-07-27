mod render;
mod resolve;

use crate::response::{self, Response};
use hyper::http::StatusCode;
use render::{render_grid, sort_items};
use resolve::resolve_directory;
use tokio::fs;

type GridResult<T> = Result<T, (StatusCode, String)>;

pub async fn handle_grid(path: Option<&str>) -> Response {
    let path = path.unwrap_or_default();

    match grid(path).await {
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
            items.push((name, file_type.is_dir()));
        }
    }

    sort_items(&mut items);

    Ok(render_grid(path, &items))
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
