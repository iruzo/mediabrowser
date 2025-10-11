use std::convert::Infallible;
use std::path::Path;
use tokio::fs;
use warp::http::StatusCode;
use percent_encoding::percent_decode_str;
use crate::types::{FileQuery, DATA_DIR};

pub async fn handle_save(query: FileQuery, body: bytes::Bytes) -> Result<impl warp::Reply, Infallible> {
    let decoded_path = percent_decode_str(&query.path).decode_utf8_lossy();
    let file_path = Path::new(&*decoded_path);

    if !file_path.starts_with(DATA_DIR) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Access denied"),
            StatusCode::FORBIDDEN,
        ));
    }

    match fs::write(&file_path, body).await {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"File saved successfully"),
            StatusCode::OK,
        )),
        Err(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"Failed to save file"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}