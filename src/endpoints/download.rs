use crate::types::{FileQuery, DATA_DIR};
use percent_encoding::percent_decode_str;
use std::convert::Infallible;
use std::path::Path;
use tokio::fs;
use tokio_util::io::ReaderStream;
use warp::hyper::Body;
use warp::{http::StatusCode, Reply};

pub async fn handle_download(query: FileQuery) -> Result<impl warp::Reply, Infallible> {
    let decoded_path = percent_decode_str(&query.path).decode_utf8_lossy();
    let file_path = Path::new(&*decoded_path);

    if !file_path.starts_with(DATA_DIR) {
        return Ok(
            warp::reply::with_status("Access denied", StatusCode::FORBIDDEN).into_response(),
        );
    }

    let file_size = match fs::metadata(&file_path).await {
        Ok(metadata) => metadata.len(),
        Err(_) => {
            return Ok(
                warp::reply::with_status("File not found", StatusCode::NOT_FOUND).into_response(),
            );
        }
    };

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
