use crate::response::{self, Response};
use crate::types::data_path;
use http_body_util::BodyExt;
use hyper::http::StatusCode;
use multer::{Constraints, Field, Multipart, SizeLimit};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

const MAX_UPLOADS: usize = 3;
const MAX_PATH_SIZE: usize = 4096;
const MAX_UPLOAD_SIZE: u64 = 256 * 1024 * 1024 * 1024;

static UPLOAD_SEMAPHORE: Semaphore = Semaphore::const_new(MAX_UPLOADS);

type UploadResult<T> = Result<T, (StatusCode, String)>;

pub async fn handle_upload(content_type: &str, body: hyper::body::Incoming) -> Response {
    let _permit = UPLOAD_SEMAPHORE.acquire().await.unwrap();

    let Ok(boundary) = multer::parse_boundary(content_type) else {
        return response::text(StatusCode::BAD_REQUEST, "Invalid multipart request");
    };
    let constraints = Constraints::new().size_limit(SizeLimit::new().whole_stream(MAX_UPLOAD_SIZE));
    let mut form = Multipart::with_constraints(body.into_data_stream(), boundary, constraints);

    let mut target_dir: Option<PathBuf> = None;
    let mut uploaded_files = 0;

    while let Some(part) = match form.next_field().await {
        Ok(part) => part,
        Err(e) => {
            return response::text(
                StatusCode::BAD_REQUEST,
                format!("Failed to process upload: {e}"),
            )
        }
    } {
        if part.name() == Some("path") {
            let path = match read_path_part(part).await {
                Ok(path) => path,
                Err((status, message)) => return response::text(status, message),
            };
            let Some(path) = data_path(path.trim()) else {
                return response::text(StatusCode::BAD_REQUEST, "Invalid upload path");
            };
            if let Err(error) = fs::create_dir_all(&path).await {
                return response::text(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to create upload directory: {error}"),
                );
            }
            target_dir = Some(path);
            continue;
        }

        if part.name() != Some("file") {
            continue;
        }

        let Some(target_dir) = target_dir.as_deref() else {
            return response::text(
                StatusCode::BAD_REQUEST,
                "Upload path must precede file fields",
            );
        };
        let Some(filename) = part.file_name().map(str::to_owned) else {
            continue;
        };
        if !valid_upload_filename(&filename) {
            return response::text(StatusCode::BAD_REQUEST, "Invalid filename");
        }

        if let Err((status, message)) = save_upload_part(part, target_dir, &filename).await {
            return response::text(status, message);
        }

        uploaded_files += 1;
    }

    if target_dir.is_none() {
        return response::text(StatusCode::BAD_REQUEST, "Upload path is required");
    }
    if uploaded_files == 0 {
        return response::text(StatusCode::BAD_REQUEST, "At least one file is required");
    }

    response::status(StatusCode::OK)
}

async fn read_path_part(mut part: Field<'_>) -> UploadResult<String> {
    let mut value = Vec::new();

    while let Some(chunk) = part.chunk().await.map_err(bad_upload_stream_error)? {
        if value.len() + chunk.len() > MAX_PATH_SIZE {
            return Err((
                StatusCode::BAD_REQUEST,
                "Upload path is too long".to_string(),
            ));
        }

        value.extend_from_slice(&chunk);
    }

    String::from_utf8(value).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            "Upload path is not UTF-8".to_string(),
        )
    })
}

async fn save_upload_part(
    mut part: Field<'_>,
    target_dir: &Path,
    filename: &str,
) -> UploadResult<()> {
    let (mut file, path) = open_upload_file(target_dir, filename)
        .await
        .map_err(save_file_error)?;

    let result = async {
        while let Some(chunk) = part.chunk().await.map_err(bad_upload_stream_error)? {
            file.write_all(&chunk).await.map_err(save_file_error)?;
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

fn save_file_error(e: std::io::Error) -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Failed to save file: {e}"),
    )
}

fn bad_upload_stream_error(e: multer::Error) -> (StatusCode, String) {
    (
        StatusCode::BAD_REQUEST,
        format!("Failed to process upload stream: {e}"),
    )
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
