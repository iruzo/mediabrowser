use crate::mime::PATH_SEGMENT;
use crate::response::{self, Response};
use crate::{resolve_path, serve_file};
use hyper::http::{HeaderMap, StatusCode};
use percent_encoding::utf8_percent_encode;
use std::fmt::Write as _;
use std::path::Path;
use tokio::fs;

struct DirectoryItem {
    name: String,
    sort_key: String,
    is_dir: bool,
}

pub async fn handle_file_server(requested_path: &str, headers: &HeaderMap) -> Response {
    let (file_path, metadata) = match resolve_path(requested_path).await {
        Ok(path) => path,
        Err(response) => return response,
    };

    if metadata.is_dir() {
        serve_directory(&file_path, requested_path).await
    } else {
        serve_file(&file_path, headers, metadata.len()).await
    }
}

async fn serve_directory(dir_path: &Path, requested_path: &str) -> Response {
    if !requested_path.is_empty() && !requested_path.ends_with('/') {
        let location = format!("/{}/", requested_path.trim_start_matches('/'));
        return hyper::http::Response::builder()
            .status(StatusCode::PERMANENT_REDIRECT)
            .header("location", location)
            .body(response::empty())
            .expect("valid directory redirect");
    }

    let mut entries = match fs::read_dir(dir_path).await {
        Ok(entries) => entries,
        Err(_) => {
            return response::text(StatusCode::INTERNAL_SERVER_ERROR, "Cannot read directory");
        }
    };

    let mut items = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        let file_type = {
            #[cfg(windows)]
            {
                fs::symlink_metadata(entry.path())
                    .await
                    .map(|metadata| metadata.file_type())
            }
            #[cfg(not(windows))]
            {
                entry.file_type().await
            }
        };

        if let Ok(file_type) = file_type {
            if file_type.is_symlink() {
                continue;
            }

            if let Some(name) = entry.file_name().to_str() {
                let is_dir = file_type.is_dir();
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
        escape_html_into(name, &mut list_items);
        if item.is_dir {
            list_items.push('/');
        }
        list_items.push_str("</a></li>\n");
    }

    let mut html = String::with_capacity(list_items.len() + display_path.len() * 2 + 128);
    let escaped_path = escape_html(display_path);
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
        escaped_path, escaped_path, list_items
    );
    html
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    escape_html_into(value, &mut escaped);
    escaped
}

fn escape_html_into(value: &str, output: &mut String) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(character),
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::{escape_html, serve_directory};
    use crate::response::Body;
    use hyper::http::StatusCode;
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn directory_listing_hides_symlinks() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("mediabrowser-httpd-{suffix}"));
        std::fs::create_dir_all(root.join("directory")).expect("create test directory");
        std::fs::write(root.join("file.txt"), []).expect("create test file");
        symlink("file.txt", root.join("file-link")).expect("create file link");
        symlink("directory", root.join("directory-link")).expect("create directory link");
        symlink("missing", root.join("broken-link")).expect("create broken link");

        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("create runtime");
        let response = runtime.block_on(serve_directory(&root, ""));
        assert_eq!(response.status(), StatusCode::OK);
        let Body::Full(Some(body)) = response.body() else {
            panic!("expected directory listing body");
        };
        let html = std::str::from_utf8(body).expect("listing is UTF-8");

        assert!(html.contains("directory/"));
        assert!(html.contains("file.txt"));
        assert!(!html.contains("file-link"));
        assert!(!html.contains("directory-link"));
        assert!(!html.contains("broken-link"));

        for path in ["folder", "nested/folder", "folder%20name", "/folder"] {
            let response = runtime.block_on(serve_directory(&root, path));
            assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
            assert_eq!(
                response.headers()["location"],
                format!("/{}/", path.trim_start_matches('/'))
            );
            assert!(matches!(response.body(), Body::Empty));
        }

        let response = runtime.block_on(serve_directory(&root, "folder/"));
        assert_eq!(response.status(), StatusCode::OK);

        std::fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn escapes_html_text() {
        assert_eq!(escape_html("<&>\"'"), "&lt;&amp;&gt;&quot;&#39;");
    }
}
