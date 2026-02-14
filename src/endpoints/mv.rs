use crate::types::{MoveQuery, DATA_DIR};
use percent_encoding::percent_decode_str;
use std::convert::Infallible;
use std::path::Path;
use tokio::fs;
use warp::http::StatusCode;

pub async fn handle_move(query: MoveQuery) -> Result<impl warp::Reply, Infallible> {
    let decoded_from = percent_decode_str(&query.from).decode_utf8_lossy();
    let decoded_to = percent_decode_str(&query.to).decode_utf8_lossy();
    let from_path = Path::new(&*decoded_from);
    let to_path = Path::new(&*decoded_to);

    if !from_path.starts_with(DATA_DIR) || !to_path.starts_with(DATA_DIR) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Access denied"),
            StatusCode::FORBIDDEN,
        ));
    }

    match fs::rename(&from_path, &to_path).await {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"Moved successfully"),
            StatusCode::OK,
        )),
        Err(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"Failed to move"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}
