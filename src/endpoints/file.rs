use std::convert::Infallible;
use std::path::Path;
use tokio::fs;
use warp::{http::StatusCode, Reply};
use percent_encoding::percent_decode_str;
use crate::types::{FileQuery, DATA_DIR};

pub async fn handle_file(query: FileQuery) -> Result<impl warp::Reply, Infallible> {
    let decoded_path = percent_decode_str(&query.path).decode_utf8_lossy();
    let file_path = Path::new(&*decoded_path);

    if !file_path.starts_with(DATA_DIR) {
        return Ok(warp::reply::with_status(
            "Access denied",
            StatusCode::FORBIDDEN,
        ).into_response());
    }

    match fs::read(&file_path).await {
        Ok(contents) => {
            let mime_type = mime_guess::from_path(&file_path)
                .first_or_octet_stream()
                .to_string();

            Ok(warp::reply::with_header(
                contents,
                "content-type",
                mime_type,
            ).into_response())
        }
        Err(_) => Ok(warp::reply::with_status(
            "File not found",
            StatusCode::NOT_FOUND,
        ).into_response()),
    }
}