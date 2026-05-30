use crate::types::{data_path, FileQuery};
use std::convert::Infallible;
use tokio::fs;
use warp::Reply;

pub async fn handle_save(
    query: FileQuery,
    body: bytes::Bytes,
) -> Result<impl warp::Reply, Infallible> {
    let Some(file_path) = data_path(&query.path) else {
        return Ok(
            warp::reply::with_status("Access denied", warp::http::StatusCode::FORBIDDEN)
                .into_response(),
        );
    };

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
