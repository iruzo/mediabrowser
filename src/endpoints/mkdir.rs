use crate::types::{data_path, FileQuery};
use percent_encoding::percent_decode_str;
use std::convert::Infallible;
use tokio::fs;
use warp::http::StatusCode;

pub async fn handle_mkdir(query: FileQuery) -> Result<impl warp::Reply, Infallible> {
    let decoded_path = percent_decode_str(&query.path).decode_utf8_lossy();
    let Some(folder_path) = data_path(decoded_path.as_ref()) else {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Access denied"),
            StatusCode::FORBIDDEN,
        ));
    };

    match fs::create_dir_all(&folder_path).await {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"Folder created successfully"),
            StatusCode::OK,
        )),
        Err(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"Failed to create folder"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}
