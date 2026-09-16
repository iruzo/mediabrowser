use crate::mime::content_type;
use crate::response::{self, Response};
use hyper::http::{HeaderMap, StatusCode};
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

pub(crate) async fn serve_file(file_path: &Path, headers: &HeaderMap, file_size: u64) -> Response {
    let mime_type = content_type(file_path);

    // Check for Range header
    if let Some(range_header) = headers.get("range") {
        if let Ok(range_str) = range_header.to_str() {
            if let Some(range) = parse_range(range_str, file_size) {
                return serve_file_range(file_path, range, file_size, mime_type).await;
            }
        }
    }

    // No range request - stream entire file
    let file = match fs::File::open(file_path).await {
        Ok(f) => f,
        Err(_) => {
            return response::text(StatusCode::NOT_FOUND, "File not found");
        }
    };

    let body = response::stream(ReaderStream::new(file));

    hyper::http::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", mime_type)
        .header("accept-ranges", "bytes")
        .header("content-length", file_size.to_string())
        .body(body)
        .expect("valid file response")
}

fn parse_range(range_str: &str, file_size: u64) -> Option<(u64, u64)> {
    // Range header format: "bytes=start-end" or "bytes=start-" or "bytes=-suffix"
    let range_str = range_str.strip_prefix("bytes=")?;

    // support single ranges, not multiple ranges
    if range_str.contains(',') {
        return None;
    }

    let (start_str, end_str) = range_str.split_once('-')?;

    match (start_str.parse::<u64>(), end_str.parse::<u64>()) {
        (Ok(start), Ok(end)) => {
            // "bytes=start-end"
            if start <= end && start < file_size {
                let end = end.min(file_size - 1);
                Some((start, end))
            } else {
                None
            }
        }
        (Ok(start), Err(_)) => {
            // "bytes=start-" (from start to end of file)
            if start < file_size {
                Some((start, file_size - 1))
            } else {
                None
            }
        }
        (Err(_), Ok(suffix)) => {
            // "bytes=-suffix" (last suffix bytes)
            if suffix > 0 && suffix <= file_size {
                Some((file_size - suffix, file_size - 1))
            } else {
                None
            }
        }
        _ => None,
    }
}

async fn serve_file_range(
    file_path: &Path,
    range: (u64, u64),
    file_size: u64,
    mime_type: &str,
) -> Response {
    let (start, end) = range;
    let content_length = end - start + 1;

    let mut file = match fs::File::open(file_path).await {
        Ok(f) => f,
        Err(_) => {
            return response::text(StatusCode::NOT_FOUND, "File not found");
        }
    };

    // Seek to start position
    if file.seek(std::io::SeekFrom::Start(start)).await.is_err() {
        return response::text(StatusCode::INTERNAL_SERVER_ERROR, "Seek failed");
    }

    let limited_reader = file.take(content_length);
    let body = response::stream(ReaderStream::new(limited_reader));

    let content_range = format!("bytes {}-{}/{}", start, end, file_size);

    hyper::http::Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header("content-type", mime_type)
        .header("accept-ranges", "bytes")
        .header("content-range", content_range)
        .header("content-length", content_length.to_string())
        .body(body)
        .expect("valid range response")
}
