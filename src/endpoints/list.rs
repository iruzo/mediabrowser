use crate::types::{api_path, data_path, ListQuery};
use percent_encoding::percent_decode_str;
use serde::Serialize;
use std::convert::Infallible;
use std::path::Path;
use std::time::UNIX_EPOCH;
use warp::http::StatusCode;
use warp::Reply;

#[derive(Serialize)]
struct ListItem {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: u64,
}

pub async fn handle_list(query: ListQuery) -> Result<warp::reply::Response, Infallible> {
    let path = query.path.unwrap_or_default();
    let decoded_path = percent_decode_str(&path).decode_utf8_lossy();
    let Some(dir_path) = data_path(decoded_path.as_ref()) else {
        return Ok(
            warp::reply::with_status("Access denied", StatusCode::FORBIDDEN).into_response(),
        );
    };

    let items = match tokio::task::spawn_blocking(move || read_items(&dir_path)).await {
        Ok(Ok(items)) => items,
        _ => {
            return Ok(
                warp::reply::with_status("Cannot read directory", StatusCode::NOT_FOUND)
                    .into_response(),
            );
        }
    };

    Ok(warp::reply::json(&items).into_response())
}

fn read_items(dir_path: &Path) -> std::io::Result<Vec<ListItem>> {
    let entries = std::fs::read_dir(dir_path)?;
    let mut items = Vec::new();

    for entry in entries.flatten() {
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        let name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(_) => continue,
        };

        items.push(ListItem {
            path: api_path(&entry.path()),
            name,
            is_dir: metadata.is_dir(),
            size: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
            modified: modified_millis(&metadata),
        });
    }

    items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(items)
}

fn modified_millis(metadata: &std::fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
