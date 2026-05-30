use crate::types::data_path;
use percent_encoding::percent_decode_str;
use std::convert::Infallible;
use tokio::fs;
use tokio_util::io::ReaderStream;
use warp::hyper::Body;
use warp::{http::StatusCode, Reply};

pub async fn handle_download(path: warp::path::Tail) -> Result<impl warp::Reply, Infallible> {
    let decoded_path = percent_decode_str(path.as_str()).decode_utf8_lossy();
    let Some(file_path) = data_path(decoded_path.as_ref()) else {
        return Ok(
            warp::reply::with_status("Access denied", StatusCode::FORBIDDEN).into_response(),
        );
    };

    let metadata = match fs::metadata(&file_path).await {
        Ok(metadata) => metadata,
        Err(_) => {
            return Ok(
                warp::reply::with_status("File not found", StatusCode::NOT_FOUND).into_response(),
            );
        }
    };

    let file_size = metadata.len();

    match fs::File::open(&file_path).await {
        Ok(file) => {
            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("download");

            let disposition = format!("attachment; filename=\"{}\"", filename);
            let stream = ReaderStream::new(file);
            let body = Body::wrap_stream(stream);

            Ok(warp::http::Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/octet-stream")
                .header("content-disposition", disposition)
                .header("content-length", file_size.to_string())
                .body(body)
                .unwrap())
        }
        Err(_) => {
            Ok(warp::reply::with_status("File not found", StatusCode::NOT_FOUND).into_response())
        }
    }
}
