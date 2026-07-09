use crate::types::{data_path, ListQuery};
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
use warp::Reply;

const MAX_UPLOADS: usize = 3;

static UPLOAD_SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();

fn get_upload_semaphore() -> &'static Semaphore {
    UPLOAD_SEMAPHORE.get_or_init(|| Semaphore::new(MAX_UPLOADS))
}

type UploadResult<T> = Result<T, (StatusCode, String)>;

pub async fn handle_upload(
    query: ListQuery,
    mut form: warp::multipart::FormData,
) -> Result<warp::reply::Response, Infallible> {
    // Acquire semaphore permit to limit concurrent uploads globally
    let _permit = get_upload_semaphore().acquire().await.unwrap();

    let target_path = query.path.unwrap_or_default();
    let decoded_path = percent_decode_str(&target_path).decode_utf8_lossy();
    let Some(target_dir) = data_path(decoded_path.as_ref()) else {
        return Ok(upload_response("Access denied", StatusCode::FORBIDDEN));
    };

    if let Err(e) = fs::create_dir_all(&target_dir).await {
        return Ok(upload_response(
            format!("Failed to create upload directory: {}", e),
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    let mut uploaded_files = 0;

    while let Some(part) = match form.try_next().await {
        Ok(part) => part,
        Err(e) => return Ok(upload_response(bad_upload_error(e), StatusCode::BAD_REQUEST)),
    } {
        if part.name() != "file" {
            continue;
        }

        let Some(filename) = part.filename().map(str::to_owned) else {
            continue;
        };
        if !valid_upload_filename(&filename) {
            return Ok(upload_response("Invalid filename", StatusCode::BAD_REQUEST));
        }

        if let Err((status, message)) = save_upload_part(part, &target_dir, &filename).await {
            return Ok(upload_response(message, status));
        }

        uploaded_files += 1;
    }

    Ok(upload_response(
        format!("Successfully uploaded {} file(s)", uploaded_files),
        StatusCode::OK,
    ))
}

async fn save_upload_part(
    part: warp::multipart::Part,
    target_dir: &Path,
    filename: &str,
) -> UploadResult<()> {
    let mut stream = part.stream();
    let mut file = open_upload_file(target_dir, filename)
        .await
        .map_err(save_file_error)?;

    while let Some(mut chunk) = stream.try_next().await.map_err(bad_upload_stream_error)? {
        write_chunk(&mut file, &mut chunk).await?;
    }

    Ok(())
}

async fn write_chunk(file: &mut tokio::fs::File, chunk: &mut impl Buf) -> UploadResult<()> {
    while chunk.has_remaining() {
        let bytes = chunk.chunk();
        if bytes.is_empty() {
            break;
        }

        file.write_all(bytes).await.map_err(save_file_error)?;
        chunk.advance(bytes.len());
    }

    Ok(())
}

fn save_file_error(e: std::io::Error) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Failed to save file: {}", e),
    )
}

fn bad_upload_error(e: warp::Error) -> String {
    format!("Failed to process upload: {}", e)
}

fn bad_upload_stream_error(e: warp::Error) -> (StatusCode, String) {
    (
        StatusCode::BAD_REQUEST,
        format!("Failed to process upload stream: {}", e),
    )
}

fn upload_response(message: impl Into<String>, status: StatusCode) -> warp::reply::Response {
    let message = message.into();
    warp::reply::with_status(warp::reply::json(&message), status).into_response()
}

fn valid_upload_filename(filename: &str) -> bool {
    if filename.is_empty() || filename.contains('/') || filename.contains('\\') {
        return false;
    }

    let mut components = Path::new(filename).components();

    matches!(
        (components.next(), components.next()),
        (Some(std::path::Component::Normal(_)), None)
    )
}

async fn open_upload_file(target_dir: &Path, filename: &str) -> std::io::Result<tokio::fs::File> {
    let original_path = target_dir.join(filename);

    match create_upload_file(&original_path).await {
        Ok(file) => Ok(file),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let timestamped_path = target_dir.join(collision_filename(filename));
            create_upload_file(&timestamped_path).await
        }
        Err(e) => Err(e),
    }
}

fn collision_filename(filename: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let suffix = nanos % 1_000_000;

    match filename.rfind('.') {
        Some(dot_pos) => format!("{}_{}{}", &filename[..dot_pos], suffix, &filename[dot_pos..]),
        None => format!("{}_{}", filename, suffix),
    }
}

async fn create_upload_file(path: &Path) -> std::io::Result<tokio::fs::File> {
    tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await
}
