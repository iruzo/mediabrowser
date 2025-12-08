use crate::types::{FileQuery, DATA_DIR};
use std::convert::Infallible;
use tokio::fs;
use warp::Reply;

pub async fn handle_save(
    query: FileQuery,
    body: bytes::Bytes,
) -> Result<impl warp::Reply, Infallible> {
    let file_path = std::path::PathBuf::from(&query.path);

    // Security check: ensure path is within DATA_DIR
    if !file_path.starts_with(DATA_DIR) {
        return Ok(
            warp::reply::with_status("Access denied", warp::http::StatusCode::FORBIDDEN)
                .into_response(),
        );
    }

    // Write file contents
    match fs::write(&file_path, body).await {
        Ok(_) => Ok(warp::reply::with_status(
            "File saved successfully",
            warp::http::StatusCode::OK,
        )
        .into_response()),
        Err(_) => Ok(warp::reply::with_status(
            "Failed to save file",
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        )
        .into_response()),
    }
}
