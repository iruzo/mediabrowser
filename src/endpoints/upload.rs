use crate::types::data_path;
use bytes::Buf;
use futures_util::TryStreamExt;
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use warp::http::StatusCode;
use warp::hyper::Body;
use warp::Reply;

const MAX_UPLOADS: usize = 3;
const MAX_PATH_SIZE: usize = 4096;

static UPLOAD_SEMAPHORE: Semaphore = Semaphore::const_new(MAX_UPLOADS);

type UploadResult<T> = Result<T, (StatusCode, String)>;

pub async fn handle_upload(
    mut form: warp::multipart::FormData,
) -> Result<warp::reply::Response, Infallible> {
    let _permit = UPLOAD_SEMAPHORE.acquire().await.unwrap();

    let mut target_dir: Option<PathBuf> = None;
    let mut uploaded_files = 0;

    while let Some(part) = match form.try_next().await {
        Ok(part) => part,
        Err(e) => {
            return Ok(upload_response(
                bad_upload_error(e),
                StatusCode::BAD_REQUEST,
            ))
        }
    } {
        if part.name() == "path" {
            let path = match read_path_part(part).await {
                Ok(path) => path,
                Err((status, message)) => return Ok(upload_response(message, status)),
            };
            let Some(path) = data_path(path.trim()) else {
                return Ok(upload_response(
                    "Invalid upload path",
                    StatusCode::BAD_REQUEST,
                ));
            };
            if let Err(error) = fs::create_dir_all(&path).await {
                return Ok(upload_response(
                    format!("Failed to create upload directory: {error}"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
            target_dir = Some(path);
            continue;
        }

        if part.name() != "file" {
            continue;
        }

        let Some(target_dir) = target_dir.as_deref() else {
            return Ok(upload_response(
                "Upload path must precede file fields",
                StatusCode::BAD_REQUEST,
            ));
        };
        let Some(filename) = part.filename().map(str::to_owned) else {
            continue;
        };
        if !valid_upload_filename(&filename) {
            return Ok(upload_response("Invalid filename", StatusCode::BAD_REQUEST));
        }

        if let Err((status, message)) = save_upload_part(part, target_dir, &filename).await {
            return Ok(upload_response(message, status));
        }

        uploaded_files += 1;
    }

    if target_dir.is_none() {
        return Ok(upload_response(
            "Upload path is required",
            StatusCode::BAD_REQUEST,
        ));
    }
    if uploaded_files == 0 {
        return Ok(upload_response(
            "At least one file is required",
            StatusCode::BAD_REQUEST,
        ));
    }

    Ok(warp::http::Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())
        .unwrap())
}

async fn read_path_part(part: warp::multipart::Part) -> UploadResult<String> {
    let mut stream = part.stream();
    let mut value = Vec::new();

    while let Some(mut chunk) = stream.try_next().await.map_err(bad_upload_stream_error)? {
        if value.len() + chunk.remaining() > MAX_PATH_SIZE {
            return Err((
                StatusCode::BAD_REQUEST,
                "Upload path is too long".to_string(),
            ));
        }

        while chunk.has_remaining() {
            let bytes = chunk.chunk();
            value.extend_from_slice(bytes);
            chunk.advance(bytes.len());
        }
    }

    String::from_utf8(value).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Upload path is not UTF-8".to_string(),
        )
    })
}

async fn save_upload_part(
    part: warp::multipart::Part,
    target_dir: &Path,
    filename: &str,
) -> UploadResult<()> {
    let mut stream = part.stream();
    let (mut file, path) = open_upload_file(target_dir, filename)
        .await
        .map_err(save_file_error)?;

    let result = async {
        while let Some(mut chunk) = stream.try_next().await.map_err(bad_upload_stream_error)? {
            write_chunk(&mut file, &mut chunk).await?;
        }

        Ok(())
    }
    .await;

    if result.is_err() {
        drop(file);
        let _ = fs::remove_file(path).await;
    }

    result
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
        format!("Failed to save file: {e}"),
    )
}

fn bad_upload_error(e: warp::Error) -> String {
    format!("Failed to process upload: {e}")
}

fn bad_upload_stream_error(e: warp::Error) -> (StatusCode, String) {
    (
        StatusCode::BAD_REQUEST,
        format!("Failed to process upload stream: {e}"),
    )
}

fn upload_response(message: impl Into<String>, status: StatusCode) -> warp::reply::Response {
    warp::reply::with_status(message.into(), status).into_response()
}

fn valid_upload_filename(filename: &str) -> bool {
    if filename.is_empty()
        || filename
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return false;
    }

    let mut components = Path::new(filename).components();

    matches!(
        (components.next(), components.next()),
        (Some(std::path::Component::Normal(_)), None)
    )
}

async fn open_upload_file(
    target_dir: &Path,
    filename: &str,
) -> std::io::Result<(tokio::fs::File, PathBuf)> {
    let mut path = target_dir.join(filename);
    let mut suffix = 0;

    loop {
        match create_upload_file(&path).await {
            Ok(file) => return Ok((file, path)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                suffix += 1;
                path = target_dir.join(collision_filename(filename, suffix));
            }
            Err(e) => return Err(e),
        }
    }
}

fn collision_filename(filename: &str, suffix: u64) -> String {
    match filename.rfind('.') {
        Some(dot_pos) => format!(
            "{}_{}{}",
            &filename[..dot_pos],
            suffix,
            &filename[dot_pos..]
        ),
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

#[cfg(test)]
mod tests {
    use super::{collision_filename, valid_upload_filename};

    #[test]
    fn validates_upload_filenames() {
        assert!(valid_upload_filename("example.txt"));
        assert!(valid_upload_filename("árbol-東京.txt"));

        for filename in ["", ".", "..", "a/b", "a\\b", "a\nb", "a\0b"] {
            assert!(!valid_upload_filename(filename));
        }
    }

    #[test]
    fn adds_collision_suffix_before_extension() {
        assert_eq!(collision_filename("example.txt", 2), "example_2.txt");
        assert_eq!(collision_filename("example", 2), "example_2");
    }
}
