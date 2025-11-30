use std::convert::Infallible;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use warp::{http::StatusCode, Reply};
use warp::http::HeaderMap;
use warp::hyper::Body;
use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};
use mime_guess::from_path;
use crate::types::DATA_DIR;

// Encode only characters that are not allowed in URL paths (similar to Apache)
const PATH_SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'#')
    .add(b'?')
    .add(b'{')
    .add(b'}');

pub async fn handle_serve(path: warp::path::Tail, headers: HeaderMap) -> Result<impl warp::Reply, Infallible> {
    let requested_path = path.as_str();
    let decoded_path = percent_decode_str(requested_path).decode_utf8_lossy();

    let file_path = if decoded_path.is_empty() || decoded_path == "/" {
        PathBuf::from(DATA_DIR)
    } else {
        PathBuf::from(DATA_DIR).join(decoded_path.as_ref())
    };

    if !file_path.starts_with(DATA_DIR) {
        return Ok(warp::reply::with_status(
            "Access denied",
            StatusCode::FORBIDDEN,
        ).into_response());
    }

    match fs::metadata(&file_path).await {
        Ok(metadata) => {
            if metadata.is_dir() {
                match serve_directory(&file_path, requested_path).await {
                    Ok(reply) => Ok(reply.into_response()),
                    Err(e) => Err(e),
                }
            } else {
                match serve_file(&file_path, &headers).await {
                    Ok(reply) => Ok(reply.into_response()),
                    Err(e) => Err(e),
                }
            }
        }
        Err(_) => {
            Ok(warp::reply::with_status(
                "Not found",
                StatusCode::NOT_FOUND,
            ).into_response())
        }
    }
}

async fn serve_file(file_path: &Path, headers: &HeaderMap) -> Result<warp::reply::Response, Infallible> {
    let file_size = match fs::metadata(file_path).await {
        Ok(metadata) => metadata.len(),
        Err(_) => {
            return Ok(warp::reply::with_status(
                "File not found",
                StatusCode::NOT_FOUND,
            ).into_response());
        }
    };

    let mime_type = from_path(file_path)
        .first_or_octet_stream()
        .to_string();

    // Check for Range header
    if let Some(range_header) = headers.get("range") {
        if let Ok(range_str) = range_header.to_str() {
            if let Some(range) = parse_range(range_str, file_size) {
                return serve_file_range(file_path, range, file_size, &mime_type).await;
            }
        }
    }

    // No range request - serve entire file
    match fs::read(file_path).await {
        Ok(contents) => {
            Ok(warp::http::Response::builder()
                .status(StatusCode::OK)
                .header("content-type", mime_type)
                .header("accept-ranges", "bytes")
                .header("content-length", file_size.to_string())
                .body(Body::from(contents))
                .unwrap())
        }
        Err(_) => {
            Ok(warp::reply::with_status(
                "File not found",
                StatusCode::NOT_FOUND,
            ).into_response())
        }
    }
}

fn parse_range(range_str: &str, file_size: u64) -> Option<(u64, u64)> {
    // Range header format: "bytes=start-end" or "bytes=start-" or "bytes=-suffix"
    let range_str = range_str.strip_prefix("bytes=")?;

    // We only support single ranges, not multiple ranges
    if range_str.contains(',') {
        return None;
    }

    let parts: Vec<&str> = range_str.split('-').collect();
    if parts.len() != 2 {
        return None;
    }

    match (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
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
) -> Result<warp::reply::Response, Infallible> {
    let (start, end) = range;
    let content_length = end - start + 1;

    let mut file = match fs::File::open(file_path).await {
        Ok(f) => f,
        Err(_) => {
            return Ok(warp::reply::with_status(
                "File not found",
                StatusCode::NOT_FOUND,
            ).into_response());
        }
    };

    // Seek to start position
    if let Err(_) = file.seek(std::io::SeekFrom::Start(start)).await {
        return Ok(warp::reply::with_status(
            "Seek failed",
            StatusCode::INTERNAL_SERVER_ERROR,
        ).into_response());
    }

    // Read the requested chunk
    let mut buffer = vec![0u8; content_length as usize];
    if let Err(_) = file.read_exact(&mut buffer).await {
        return Ok(warp::reply::with_status(
            "Read failed",
            StatusCode::INTERNAL_SERVER_ERROR,
        ).into_response());
    }

    let content_range = format!("bytes {}-{}/{}", start, end, file_size);

    Ok(warp::http::Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header("content-type", mime_type)
        .header("accept-ranges", "bytes")
        .header("content-range", content_range)
        .header("content-length", content_length.to_string())
        .body(Body::from(buffer))
        .unwrap())
}

async fn serve_directory(dir_path: &Path, requested_path: &str) -> Result<impl warp::Reply, Infallible> {
    let mut entries = match fs::read_dir(dir_path).await {
        Ok(entries) => entries,
        Err(_) => {
            return Ok(warp::reply::with_status(
                "Cannot read directory",
                StatusCode::INTERNAL_SERVER_ERROR,
            ).into_response());
        }
    };

    let mut items = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Ok(metadata) = entry.metadata().await {
            if let Some(name) = entry.file_name().to_str() {
                let is_dir = metadata.is_dir();
                let size = if is_dir { 0 } else { metadata.len() };

                items.push((name.to_string(), is_dir, size));
            }
        }
    }

    items.sort_by(|a, b| {
        match (a.1, b.1) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.0.to_lowercase().cmp(&b.0.to_lowercase()),
        }
    });

    let display_path = if requested_path.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", requested_path.trim_start_matches('/'))
    };
    let html = generate_directory_listing(&display_path, &items);

    Ok(warp::reply::with_header(
        warp::reply::html(html),
        "content-type",
        "text/html; charset=utf-8",
    ).into_response())
}

fn generate_directory_listing(path: &str, items: &[(String, bool, u64)]) -> String {
    let display_path = if path.is_empty() || path == "/" {
        "/"
    } else {
        path
    };

    let mut list_items = String::new();

    for (name, is_dir, _size) in items {
        let display_name = if *is_dir {
            format!("{}/", name)
        } else {
            name.clone()
        };

        let encoded_name = utf8_percent_encode(name, PATH_SEGMENT).to_string();
        let url = if *is_dir {
            format!("{}/", encoded_name)
        } else {
            encoded_name
        };

        list_items.push_str(&format!(
            r#"<li><a href="{}"> {}</a></li>
"#,
            url, display_name
        ));
    }

    format!(r#"<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 3.2 Final//EN">
<html>
 <head>
  <title>Index of {}</title>
 </head>
 <body>
<h1>Index of {}</h1>
<ul>{}</ul>
</body></html>"#, display_path, display_path, list_items)
}
