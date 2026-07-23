use crate::response::{self, Response};
use crate::types::data_path;
use hyper::http::{HeaderMap, StatusCode};
use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};
use std::fmt::Write as _;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

// Encode only characters that are not allowed in URL paths (similar to Apache)
pub(crate) const PATH_SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'#')
    .add(b'?')
    .add(b'{')
    .add(b'}');

struct DirectoryItem {
    name: String,
    sort_key: String,
    is_dir: bool,
}

pub async fn handle_file_server(requested_path: &str, headers: &HeaderMap) -> Response {
    let decoded_path = percent_decode_str(requested_path).decode_utf8_lossy();

    let Some(file_path) = data_path(decoded_path.as_ref()) else {
        return response::text(StatusCode::FORBIDDEN, "Access denied");
    };

    let metadata = match fs::metadata(&file_path).await {
        Ok(metadata) => metadata,
        Err(_) => {
            return response::text(StatusCode::NOT_FOUND, "Not found");
        }
    };

    if metadata.is_dir() {
        serve_directory(&file_path, requested_path).await
    } else {
        serve_file(&file_path, headers, metadata.len()).await
    }
}

async fn serve_file(file_path: &Path, headers: &HeaderMap, file_size: u64) -> Response {
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

pub(crate) fn content_type(path: &Path) -> &'static str {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return "application/octet-stream";
    };

    match extension.to_ascii_lowercase().as_str() {
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "mpg" | "mpeg" => "video/mpeg",
        "ts" => "video/mp2t",
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "ogg" => "audio/ogg",
        "opus" => "audio/opus",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "avif" => "image/avif",
        "vtt" => "text/vtt",
        "srt" | "txt" | "md" | "log" => "text/plain",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

fn parse_range(range_str: &str, file_size: u64) -> Option<(u64, u64)> {
    // Range header format: "bytes=start-end" or "bytes=start-" or "bytes=-suffix"
    let range_str = range_str.strip_prefix("bytes=")?;

    // We only support single ranges, not multiple ranges
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

async fn serve_directory(dir_path: &Path, requested_path: &str) -> Response {
    let mut entries = match fs::read_dir(dir_path).await {
        Ok(entries) => entries,
        Err(_) => {
            return response::text(StatusCode::INTERNAL_SERVER_ERROR, "Cannot read directory");
        }
    };

    let mut items = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Ok(metadata) = entry.metadata().await {
            if let Some(name) = entry.file_name().to_str() {
                let is_dir = metadata.is_dir();
                let name = name.to_string();
                let sort_key = name.to_lowercase();

                items.push(DirectoryItem {
                    name,
                    sort_key,
                    is_dir,
                });
            }
        }
    }

    items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.sort_key.cmp(&b.sort_key),
    });

    let display_path = if requested_path.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", requested_path.trim_start_matches('/'))
    };
    let html = generate_directory_listing(&display_path, &items);

    hyper::http::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html; charset=utf-8")
        .body(response::full(html))
        .expect("valid listing response")
}

fn generate_directory_listing(path: &str, items: &[DirectoryItem]) -> String {
    let display_path = if path.is_empty() || path == "/" {
        "/"
    } else {
        path
    };

    let mut list_items = String::with_capacity(items.len() * 48);

    for item in items {
        let name = &item.name;
        let encoded_name = utf8_percent_encode(name, PATH_SEGMENT).to_string();
        list_items.push_str(r#"<li><a href=""#);
        list_items.push_str(&encoded_name);
        if item.is_dir {
            list_items.push('/');
        }
        list_items.push_str(r#""> "#);
        list_items.push_str(name);
        if item.is_dir {
            list_items.push('/');
        }
        list_items.push_str("</a></li>\n");
    }

    let mut html = String::with_capacity(list_items.len() + display_path.len() * 2 + 128);
    let _ = write!(
        html,
        r#"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 3.2 Final//EN">
<html>
 <head>
  <title>Index of {}</title>
 </head>
 <body>
<h1>Index of {}</h1>
<ul>{}</ul>
</body></html>"#,
        display_path, display_path, list_items
    );
    html
}

#[cfg(test)]
mod tests {
    use super::content_type;
    use std::path::Path;

    #[test]
    fn maps_extensions_to_content_types() {
        assert_eq!(content_type(Path::new("movie.mp4")), "video/mp4");
        assert_eq!(content_type(Path::new("MOVIE.MKV")), "video/x-matroska");
        assert_eq!(content_type(Path::new("song.flac")), "audio/flac");
        assert_eq!(content_type(Path::new("photo.jpeg")), "image/jpeg");
        assert_eq!(content_type(Path::new("notes.txt")), "text/plain");
        assert_eq!(
            content_type(Path::new("unknown.xyz")),
            "application/octet-stream"
        );
        assert_eq!(
            content_type(Path::new("no-extension")),
            "application/octet-stream"
        );
    }
}
