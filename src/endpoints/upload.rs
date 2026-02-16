use crate::types::{ListQuery, DATA_DIR};
use bytes::Buf;
use futures_util::TryStreamExt;
use percent_encoding::percent_decode_str;
use std::convert::Infallible;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use warp::http::StatusCode;

// Global semaphore to limit concurrent uploads to 3
static UPLOAD_SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();

// Use 3 for better HDD support
fn get_upload_semaphore() -> &'static Semaphore {
    UPLOAD_SEMAPHORE.get_or_init(|| Semaphore::new(3))
}

pub async fn handle_upload(
    query: ListQuery,
    mut form: warp::multipart::FormData,
) -> Result<impl warp::Reply, Infallible> {
    // Acquire semaphore permit to limit concurrent uploads globally
    let _permit = get_upload_semaphore().acquire().await.unwrap();

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
                        let mut stream = part.stream();

                        let mut file = match open_upload_file(&target_dir, &filename).await {
                            Ok(file) => file,
                            Err(e) => {
                                return Ok(warp::reply::with_status(
                                    warp::reply::json(&format!("Failed to save file: {}", e)),
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                ));
                            }
                        };

                        loop {
                            match stream.try_next().await {
                                Ok(Some(mut chunk)) => {
                                    while chunk.has_remaining() {
                                        let bytes = chunk.chunk();
                                        if bytes.is_empty() {
                                            break;
                                        }

                                        if let Err(e) = file.write_all(bytes).await {
                                            return Ok(warp::reply::with_status(
                                                warp::reply::json(&format!(
                                                    "Failed to save file: {}",
                                                    e
                                                )),
                                                StatusCode::INTERNAL_SERVER_ERROR,
                                            ));
                                        }

                                        let len = bytes.len();
                                        chunk.advance(len);
                                    }
                                }
                                Ok(None) => {
                                    uploaded_files += 1;
                                    break;
                                }
                                Err(e) => {
                                    return Ok(warp::reply::with_status(
                                        warp::reply::json(&format!(
                                            "Failed to process upload stream: {}",
                                            e
                                        )),
                                        StatusCode::BAD_REQUEST,
                                    ));
                                }
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

async fn open_upload_file(target_dir: &Path, filename: &str) -> std::io::Result<tokio::fs::File> {
    use tokio::fs::OpenOptions;

    let original_path = target_dir.join(filename);

    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&original_path)
        .await
    {
        Ok(file) => Ok(file),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let suffix = nanos % 1_000_000;

            let timestamped_filename = if let Some(dot_pos) = filename.rfind('.') {
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
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(timestamped_path)
                .await
        }
        Err(e) => Err(e),
    }
}
