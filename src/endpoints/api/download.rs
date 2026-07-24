use crate::response::{self, Response};
use crate::types::{data_dir, data_path};
use hyper::http::StatusCode;
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use std::path::PathBuf;
use tokio::fs;
use tokio_util::io::ReaderStream;

const MAX_PATH_SIZE: usize = 4096;

type DownloadResult<T> = Result<T, (StatusCode, String)>;

pub async fn handle_download(path: &str) -> Response {
    match download(path).await {
        Ok(response) => response,
        Err((status, message)) => response::text(status, message),
    }
}

async fn download(path: &str) -> DownloadResult<Response> {
    let path = download_path(path)?;
    let root = fs::canonicalize(data_dir()).await.map_err(download_error)?;
    let path = fs::canonicalize(path).await.map_err(download_error)?;

    if !path.starts_with(&root) {
        return Err((
            StatusCode::FORBIDDEN,
            "path escapes data directory".to_string(),
        ));
    }

    let file = fs::File::open(&path).await.map_err(download_error)?;
    let metadata = file.metadata().await.map_err(download_error)?;

    if !metadata.is_file() {
        return Err((
            StatusCode::BAD_REQUEST,
            "path is not a regular file".to_string(),
        ));
    }

    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download");
    let disposition = content_disposition(filename);
    let body = response::stream(ReaderStream::new(file));

    Ok(hyper::http::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/octet-stream")
        .header("content-disposition", disposition)
        .header("content-length", metadata.len().to_string())
        .body(body)
        .unwrap())
}

fn download_path(path: &str) -> DownloadResult<PathBuf> {
    if !valid_percent_encoding(path) {
        return Err((StatusCode::BAD_REQUEST, "invalid encoded path".to_string()));
    }

    let path = percent_decode_str(path)
        .decode_utf8()
        .map_err(|_| (StatusCode::BAD_REQUEST, "path is not UTF-8".to_string()))?;

    if path.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "path is required".to_string()));
    }
    if path.len() > MAX_PATH_SIZE || path.chars().any(|c| c.is_control() || matches!(c, '\\')) {
        return Err((StatusCode::BAD_REQUEST, "invalid path".to_string()));
    }

    let path = data_path(path.as_ref())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))?;

    if path.as_path() == data_dir() {
        return Err((
            StatusCode::BAD_REQUEST,
            "path must identify a file".to_string(),
        ));
    }

    Ok(path)
}

fn valid_percent_encoding(path: &str) -> bool {
    let bytes = path.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != b'%' {
            i += 1;
            continue;
        }
        if i + 2 >= bytes.len()
            || !bytes[i + 1].is_ascii_hexdigit()
            || !bytes[i + 2].is_ascii_hexdigit()
        {
            return false;
        }

        i += 3;
    }

    true
}

fn content_disposition(filename: &str) -> String {
    let filename = utf8_percent_encode(filename, NON_ALPHANUMERIC);
    format!("attachment; filename=\"download\"; filename*=UTF-8''{filename}")
}

fn download_error(error: std::io::Error) -> (StatusCode, String) {
    if error.kind() == std::io::ErrorKind::NotFound {
        (StatusCode::NOT_FOUND, "file not found".to_string())
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to download file: {error}"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{content_disposition, download_path, valid_percent_encoding, MAX_PATH_SIZE};
    use hyper::http::StatusCode;

    #[test]
    fn validates_download_paths() {
        for path in [
            "file.txt",
            "folder/file.txt",
            "%C3%A1rbol/%E6%9D%B1%E4%BA%AC.txt",
        ] {
            assert!(download_path(path).is_ok());
        }

        assert_eq!(
            download_path(""),
            Err((StatusCode::BAD_REQUEST, "path is required".to_string()))
        );

        for path in ["/", "%2F", ".", "./", "/./"] {
            assert_eq!(
                download_path(path),
                Err((
                    StatusCode::BAD_REQUEST,
                    "path must identify a file".to_string()
                ))
            );
        }

        for path in ["..", "../file", "%2E%2E/file", "a%5Cb", "a%0Ab"] {
            assert!(download_path(path).is_err());
        }

        assert_eq!(
            download_path(&"a".repeat(MAX_PATH_SIZE + 1)),
            Err((StatusCode::BAD_REQUEST, "invalid path".to_string()))
        );
    }

    #[test]
    fn rejects_malformed_percent_encoding() {
        for path in ["%", "%2", "%GG", "file%2Gtxt"] {
            assert!(!valid_percent_encoding(path));
            assert!(download_path(path).is_err());
        }
    }

    #[test]
    fn encodes_download_filename() {
        let disposition = content_disposition("á\".txt");

        assert_eq!(
            disposition,
            "attachment; filename=\"download\"; filename*=UTF-8''%C3%A1%22%2Etxt"
        );
    }
}
