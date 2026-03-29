use crate::types::{ListQuery, DATA_DIR};
use percent_encoding::percent_decode_str;
use serde::Serialize;
use std::convert::Infallible;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use tokio::fs;
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
    let path = query.path.unwrap_or_else(|| DATA_DIR.to_string());
    let decoded_path = percent_decode_str(&path).decode_utf8_lossy();
    let dir_path = PathBuf::from(decoded_path.as_ref());

    if !dir_path.starts_with(DATA_DIR) {
        return Ok(
            warp::reply::with_status("Access denied", StatusCode::FORBIDDEN).into_response(),
        );
    }

    let mut entries = match fs::read_dir(&dir_path).await {
        Ok(entries) => entries,
        Err(_) => {
            return Ok(
                warp::reply::with_status("Cannot read directory", StatusCode::NOT_FOUND)
                    .into_response(),
            );
        }
    };

    let mut items = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        let metadata = match entry.metadata().await {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        let name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(_) => continue,
        };

        items.push(ListItem {
            path: entry.path().to_string_lossy().into_owned(),
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

    Ok(warp::reply::json(&items).into_response())
}

fn modified_millis(metadata: &std::fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
