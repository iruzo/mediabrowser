use crate::types::{SearchQuery, DATA_DIR};
use percent_encoding::percent_decode_str;
use serde::Serialize;
use std::cmp::Ordering;
use std::convert::Infallible;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;
use warp::http::StatusCode;
use warp::Reply;

const MAX_SEARCH_RESULTS: usize = 500;

#[derive(Serialize)]
struct SearchItem {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: u64,
}

pub async fn handle_search(query: SearchQuery) -> Result<warp::reply::Response, Infallible> {
    let path = query.path.unwrap_or_else(|| DATA_DIR.to_string());
    let search = query.query.unwrap_or_default();

    let decoded_path = percent_decode_str(&path).decode_utf8_lossy();
    let dir_path = PathBuf::from(decoded_path.as_ref());

    if !dir_path.starts_with(DATA_DIR) {
        return Ok(
            warp::reply::with_status("Access denied", StatusCode::FORBIDDEN).into_response(),
        );
    }

    let terms: Vec<String> = search
        .split_whitespace()
        .map(|term| term.to_lowercase())
        .filter(|term| !term.is_empty())
        .collect();

    if terms.is_empty() {
        let items: Vec<SearchItem> = Vec::new();
        return Ok(warp::reply::json(&items).into_response());
    }

    if !dir_path.is_dir() {
        return Ok(
            warp::reply::with_status("Cannot read directory", StatusCode::NOT_FOUND).into_response(),
        );
    }

    let mut items = Vec::new();

    for entry in WalkDir::new(&dir_path)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.starts_with(DATA_DIR) {
            continue;
        }

        let relative = path
            .strip_prefix(&dir_path)
            .ok()
            .map(|value| value.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if !terms.iter().all(|term| relative.contains(term)) {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        items.push(SearchItem {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: path.to_string_lossy().into_owned(),
            is_dir: metadata.is_dir(),
            size: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
            modified: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(0),
        });

        if items.len() >= MAX_SEARCH_RESULTS {
            break;
        }
    }

    items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => a.path.to_lowercase().cmp(&b.path.to_lowercase()),
    });

    Ok(warp::reply::json(&items).into_response())
}
