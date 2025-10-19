use std::convert::Infallible;
use std::path::Path;
use tokio::fs;
use warp::http::StatusCode;
use percent_encoding::percent_decode_str;
use futures_util::TryStreamExt;
use bytes::Buf;
use crate::types::{ListQuery, DATA_DIR};

pub async fn handle_upload(query: ListQuery, mut form: warp::multipart::FormData) -> Result<impl warp::Reply, Infallible> {
    let target_path = query.path.unwrap_or_else(|| DATA_DIR.to_string());
    let decoded_path = percent_decode_str(&target_path).decode_utf8_lossy();
    let target_dir = Path::new(&*decoded_path);

    if !target_dir.starts_with(DATA_DIR) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Access denied"),
            StatusCode::FORBIDDEN,
        ));
    }

    if let Err(e) = fs::create_dir_all(&target_dir).await {
        return Ok(warp::reply::with_status(
            warp::reply::json(&format!("Failed to create upload directory: {}", e)),
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    let mut uploaded_files = 0;

    loop {
        match form.try_next().await {
            Ok(Some(part)) => {
                let name = part.name();

                if name == "file" {
                    if let Some(filename) = part.filename() {
                        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
                        let timestamped_filename = format!("{}_{}", timestamp, filename);
                        let file_path = target_dir.join(timestamped_filename);

                        let mut bytes = Vec::new();
                        let mut stream = part.stream();

                        while let Some(chunk) = stream.try_next().await.unwrap_or(None) {
                            bytes.extend_from_slice(chunk.chunk());
                        }

                        match fs::write(&file_path, &bytes).await {
                            Ok(_) => {
                                uploaded_files += 1;
                            }
                            Err(e) => {
                                return Ok(warp::reply::with_status(
                                    warp::reply::json(&format!("Failed to save file: {}", e)),
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                ));
                            }
                        }
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                return Ok(warp::reply::with_status(
                    warp::reply::json(&format!("Failed to process upload: {}", e)),
                    StatusCode::BAD_REQUEST,
                ));
            }
        }
    }

    Ok(warp::reply::with_status(
        warp::reply::json(&format!("Successfully uploaded {} file(s)", uploaded_files)),
        StatusCode::OK,
    ))
}
