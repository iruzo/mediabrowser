use std::convert::Infallible;
use std::path::Path;
use tokio::fs;
use warp::http::StatusCode;
use percent_encoding::percent_decode_str;
use crate::types::{FileInfo, ListQuery, DATA_DIR};

pub async fn handle_list(query: ListQuery) -> Result<impl warp::Reply, Infallible> {
    let path = query.path.unwrap_or_else(|| DATA_DIR.to_string());
    let decoded_path = percent_decode_str(&path).decode_utf8_lossy();

    match load_directory(&decoded_path).await {
        Ok(files) => Ok(warp::reply::with_status(
            warp::reply::json(&files),
            StatusCode::OK,
        )),
        Err(_) => Ok(warp::reply::with_status(
            warp::reply::json(&Vec::<FileInfo>::new()),
            StatusCode::NOT_FOUND,
        )),
    }
}

async fn load_directory(path: &str) -> Result<Vec<FileInfo>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    let mut entries = fs::read_dir(path).await?;

    // Add parent directory entry if not at root
    if path != DATA_DIR && path.starts_with(DATA_DIR) {
        if let Some(parent) = Path::new(path).parent() {
            files.push(FileInfo {
                name: "..".to_string(),
                path: parent.to_string_lossy().to_string(),
                is_dir: true,
                size: 0,
                modified: "".to_string(),
                file_type: "directory".to_string(),
            });
        }
    }

    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        let file_path = entry.path().to_string_lossy().to_string();

        let file_type = if metadata.is_dir() {
            "directory".to_string()
        } else {
            determine_file_type(&file_name)
        };

        let modified = metadata
            .modified()
            .map(|time| {
                use std::time::UNIX_EPOCH;
                let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
                let secs = duration.as_secs();
                let datetime = chrono::DateTime::from_timestamp(secs as i64, 0);
                datetime.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
            })
            .unwrap_or_else(|_| "Unknown".to_string());

        files.push(FileInfo {
            name: file_name,
            path: file_path,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified,
            file_type,
        });
    }

    files.sort_by(|a, b| {
        if a.name == ".." {
            std::cmp::Ordering::Less
        } else if b.name == ".." {
            std::cmp::Ordering::Greater
        } else if a.is_dir == b.is_dir {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        } else if a.is_dir {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    Ok(files)
}

fn determine_file_type(filename: &str) -> String {
    let ext = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" | "ico" => "image".to_string(),
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "ogv" => "video".to_string(),
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" => "audio".to_string(),
        "txt" | "md" | "rs" | "js" | "ts" | "html" | "css" | "json" | "xml" | "log" | "py" | "java" | "c" | "cpp" | "h" => "text".to_string(),
        "pdf" => "pdf".to_string(),
        "zip" | "rar" | "tar" | "gz" | "7z" => "archive".to_string(),
        _ => "unknown".to_string(),
    }
}