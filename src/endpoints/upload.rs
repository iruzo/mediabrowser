use crate::types::{ListQuery, DATA_DIR};
use bytes::Buf;
use futures_util::TryStreamExt;
use percent_encoding::percent_decode_str;
use std::convert::Infallible;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use warp::http::StatusCode;

pub async fn handle_upload(
    query: ListQuery,
    mut form: warp::multipart::FormData,
) -> Result<impl warp::Reply, Infallible> {
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
                        let filename = filename.to_string();
                        let file_path = target_dir.join(&filename);

                        let mut bytes = Vec::new();
                        let mut stream = part.stream();

                        while let Some(chunk) = stream.try_next().await.unwrap_or(None) {
                            bytes.extend_from_slice(chunk.chunk());
                        }

                        // Try to create file exclusively (fails if exists)
                        use tokio::fs::OpenOptions;
                        let write_result = OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(&file_path)
                            .await;

                        match write_result {
                            Ok(mut file) => {
                                // File didn't exist, write with original name
                                if let Err(e) = file.write_all(&bytes).await {
                                    return Ok(warp::reply::with_status(
                                        warp::reply::json(&format!("Failed to save file: {}", e)),
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                    ));
                                }
                                uploaded_files += 1;
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                                // File exists, retry with nanosecond suffix
                                let nanos = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_nanos();
                                let suffix = nanos % 1_000_000;

                                let timestamped_filename =
                                    if let Some(dot_pos) = filename.rfind('.') {
                                        format!(
                                            "{}_{}{}",
                                            &filename[..dot_pos],
                                            suffix,
                                            &filename[dot_pos..]
                                        )
                                    } else {
                                        format!("{}_{}", filename, suffix)
                                    };

                                let timestamped_path = target_dir.join(timestamped_filename);
                                match fs::write(&timestamped_path, &bytes).await {
                                    Ok(_) => {
                                        uploaded_files += 1;
                                    }
                                    Err(e) => {
                                        return Ok(warp::reply::with_status(
                                            warp::reply::json(&format!(
                                                "Failed to save file: {}",
                                                e
                                            )),
                                            StatusCode::INTERNAL_SERVER_ERROR,
                                        ));
                                    }
                                }
                            }
                            Err(e) => {
                                // Other error (permissions, disk full, etc.)
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
