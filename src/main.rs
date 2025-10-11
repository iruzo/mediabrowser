use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::path::Path;
use tokio::fs;
use warp::{Filter, Reply};
use warp::http::StatusCode;
use percent_encoding::percent_decode_str;
use futures_util::TryStreamExt;
use bytes::Buf;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileInfo {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: String,
    file_type: String,
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FileQuery {
    path: String,
}

const DATA_DIR: &str = "/data";
const PORT: u16 = 8080;

#[tokio::main]
async fn main() {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type", "accept", "authorization", "x-requested-with"])
        .allow_methods(vec!["GET", "POST", "DELETE", "OPTIONS"]);

    let static_files = warp::path::end()
        .map(|| warp::reply::html(include_str!("../static/index.html")));

    let api_list = warp::path("api")
        .and(warp::path("list"))
        .and(warp::get())
        .and(warp::query::<ListQuery>())
        .and_then(handle_list);

    let api_file = warp::path("api")
        .and(warp::path("file"))
        .and(warp::get().or(warp::head()).unify())
        .and(warp::query::<FileQuery>())
        .and_then(handle_file);

    let api_download = warp::path("api")
        .and(warp::path("download"))
        .and(warp::get())
        .and(warp::query::<FileQuery>())
        .and_then(handle_download);

    let api_upload = warp::path("api")
        .and(warp::path("upload"))
        .and(warp::post())
        .and(warp::query::<ListQuery>())
        .and(warp::body::content_length_limit(1024 * 1024 * 100)) // 100MB limit
        .and(warp::multipart::form().max_length(1024 * 1024 * 100))
        .and_then(handle_upload);

    let api_delete = warp::path("api")
        .and(warp::path("delete"))
        .and(warp::delete())
        .and(warp::query::<FileQuery>())
        .and_then(handle_delete);

    let api_mkdir = warp::path("api")
        .and(warp::path("mkdir"))
        .and(warp::post())
        .and(warp::query::<FileQuery>())
        .and_then(handle_mkdir);

    let api_save = warp::path("api")
        .and(warp::path("save"))
        .and(warp::post())
        .and(warp::query::<FileQuery>())
        .and(warp::body::content_length_limit(1024 * 1024 * 10)) // 10MB limit for text files
        .and(warp::body::bytes())
        .and_then(handle_save);

    let routes = static_files
        .or(api_list)
        .or(api_file)
        .or(api_download)
        .or(api_upload)
        .or(api_delete)
        .or(api_mkdir)
        .or(api_save)
        .with(cors);

    println!("File manager server starting on http://0.0.0.0:{}", PORT);
    println!("Serving files from: {}", DATA_DIR);

    warp::serve(routes)
        .run(([0, 0, 0, 0], PORT))
        .await;
}

async fn handle_list(query: ListQuery) -> Result<impl warp::Reply, Infallible> {
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

async fn handle_file(query: FileQuery) -> Result<impl warp::Reply, Infallible> {
    let decoded_path = percent_decode_str(&query.path).decode_utf8_lossy();
    let file_path = Path::new(&*decoded_path);

    if !file_path.starts_with(DATA_DIR) {
        return Ok(warp::reply::with_status(
            "Access denied",
            StatusCode::FORBIDDEN,
        ).into_response());
    }

    match fs::read(&file_path).await {
        Ok(contents) => {
            let mime_type = mime_guess::from_path(&file_path)
                .first_or_octet_stream()
                .to_string();

            Ok(warp::reply::with_header(
                contents,
                "content-type",
                mime_type,
            ).into_response())
        }
        Err(_) => Ok(warp::reply::with_status(
            "File not found",
            StatusCode::NOT_FOUND,
        ).into_response()),
    }
}

async fn handle_download(query: FileQuery) -> Result<impl warp::Reply, Infallible> {
    let decoded_path = percent_decode_str(&query.path).decode_utf8_lossy();
    let file_path = Path::new(&*decoded_path);

    if !file_path.starts_with(DATA_DIR) {
        return Ok(warp::reply::with_status(
            "Access denied",
            StatusCode::FORBIDDEN,
        ).into_response());
    }

    match fs::read(&file_path).await {
        Ok(contents) => {
            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("download");

            let disposition = format!("attachment; filename=\"{}\"", filename);

            Ok(warp::reply::with_header(
                warp::reply::with_header(contents, "content-type", "application/octet-stream"),
                "content-disposition",
                disposition,
            ).into_response())
        }
        Err(_) => Ok(warp::reply::with_status(
            "File not found",
            StatusCode::NOT_FOUND,
        ).into_response()),
    }
}

async fn handle_upload(query: ListQuery, mut form: warp::multipart::FormData) -> Result<impl warp::Reply, Infallible> {
    let target_path = query.path.unwrap_or_else(|| DATA_DIR.to_string());
    let decoded_path = percent_decode_str(&target_path).decode_utf8_lossy();
    let target_dir = Path::new(&*decoded_path);

    if !target_dir.starts_with(DATA_DIR) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Access denied"),
            StatusCode::FORBIDDEN,
        ));
    }

    // Ensure target directory exists
    if let Err(e) = fs::create_dir_all(&target_dir).await {
        return Ok(warp::reply::with_status(
            warp::reply::json(&format!("Failed to create upload directory: {}", e)),
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    let mut uploaded_files = 0;

    while let Ok(Some(part)) = form.try_next().await {
        let name = part.name();

        if name == "file" {
            if let Some(filename) = part.filename() {
                // Add timestamp to filename to prevent overwrites
                let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
                let timestamped_filename = format!("{}_{}", timestamp, filename);
                let file_path = target_dir.join(timestamped_filename);

                // Collect bytes from stream
                let mut bytes = Vec::new();
                let mut stream = part.stream();

                while let Ok(chunk) = stream.try_next().await {
                    if let Some(chunk) = chunk {
                        bytes.extend_from_slice(chunk.chunk());
                    } else {
                        break;
                    }
                }

                match fs::write(&file_path, &bytes).await {
                    Ok(_) => {
                        uploaded_files += 1;
                    }
                    Err(e) => {
                        return Ok(warp::reply::with_status(
                            warp::reply::json(&format!("Failed to save file: {}", e)),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        ));
                    }
                }
            }
        }
    }


    Ok(warp::reply::with_status(
        warp::reply::json(&format!("Successfully uploaded {} file(s)", uploaded_files)),
        StatusCode::OK,
    ))
}

async fn handle_delete(query: FileQuery) -> Result<impl warp::Reply, Infallible> {
    let decoded_path = percent_decode_str(&query.path).decode_utf8_lossy();
    let file_path = Path::new(&*decoded_path);

    if !file_path.starts_with(DATA_DIR) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Access denied"),
            StatusCode::FORBIDDEN,
        ));
    }

    let result = if file_path.is_dir() {
        fs::remove_dir_all(&file_path).await
    } else {
        fs::remove_file(&file_path).await
    };

    match result {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"Deleted successfully"),
            StatusCode::OK,
        )),
        Err(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"Failed to delete"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

async fn handle_mkdir(query: FileQuery) -> Result<impl warp::Reply, Infallible> {
    let decoded_path = percent_decode_str(&query.path).decode_utf8_lossy();
    let folder_path = Path::new(&*decoded_path);

    if !folder_path.starts_with(DATA_DIR) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Access denied"),
            StatusCode::FORBIDDEN,
        ));
    }

    match fs::create_dir_all(&folder_path).await {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"Folder created successfully"),
            StatusCode::OK,
        )),
        Err(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"Failed to create folder"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

async fn handle_save(query: FileQuery, body: bytes::Bytes) -> Result<impl warp::Reply, Infallible> {
    let decoded_path = percent_decode_str(&query.path).decode_utf8_lossy();
    let file_path = Path::new(&*decoded_path);

    if !file_path.starts_with(DATA_DIR) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"Access denied"),
            StatusCode::FORBIDDEN,
        ));
    }

    match fs::write(&file_path, body).await {
        Ok(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"File saved successfully"),
            StatusCode::OK,
        )),
        Err(_) => Ok(warp::reply::with_status(
            warp::reply::json(&"Failed to save file"),
            StatusCode::INTERNAL_SERVER_ERROR,
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
