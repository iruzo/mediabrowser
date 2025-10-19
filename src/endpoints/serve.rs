use std::convert::Infallible;
use std::path::{Path, PathBuf};
use tokio::fs;
use warp::{http::StatusCode, Reply};
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

pub async fn handle_serve(path: warp::path::Tail) -> Result<impl warp::Reply, Infallible> {
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
                match serve_file(&file_path).await {
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

async fn serve_file(file_path: &Path) -> Result<impl warp::Reply, Infallible> {
    match fs::read(file_path).await {
        Ok(contents) => {
            let mime_type = from_path(file_path)
                .first_or_octet_stream()
                .to_string();

            Ok(warp::reply::with_header(
                contents,
                "content-type",
                mime_type,
            ).into_response())
        }
        Err(_) => {
            Ok(warp::reply::with_status(
                "File not found",
                StatusCode::NOT_FOUND,
            ).into_response())
        }
    }
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
