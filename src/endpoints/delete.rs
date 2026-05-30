use crate::types::{data_path, FileQuery};
use percent_encoding::percent_decode_str;
use std::convert::Infallible;
use tokio::fs;
use warp::http::StatusCode;

pub async fn handle_delete(query: FileQuery) -> Result<impl warp::Reply, Infallible> {
    let decoded_path = percent_decode_str(&query.path).decode_utf8_lossy();
    let Some(file_path) = data_path(decoded_path.as_ref()) else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Access denied"),
            StatusCode::FORBIDDEN,
        ));
    };

    let result = if file_path.is_dir() {
        fs::remove_dir_all(&file_path).await
    } else {
        fs::remove_file(&file_path).await
    };

    match result {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"Deleted successfully"),
            StatusCode::OK,
        )),
        Err(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"Failed to delete"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}
