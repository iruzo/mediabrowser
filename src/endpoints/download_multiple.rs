use crate::types::DATA_DIR;
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use std::convert::Infallible;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio_util::io::ReaderStream;
use warp::hyper::Body;
use warp::{http::StatusCode, Reply};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[derive(Deserialize)]
pub struct DownloadMultipleQuery {
    pub paths: String,
}

pub async fn handle_download_multiple(
    query: DownloadMultipleQuery,
) -> Result<impl warp::Reply, Infallible> {
    let paths: Vec<String> = query
        .paths
        .split(',')
        .map(|s| percent_decode_str(s.trim()).decode_utf8_lossy().to_string())
        .collect();

    if paths.is_empty() {
        return Ok(
            warp::reply::with_status("No files specified", StatusCode::BAD_REQUEST).into_response(),
        );
    }

    let temp_zip_path = create_temp_zip_path();
    let zip_result = tokio::task::spawn_blocking({
        let paths = paths;
        let temp_zip_path = temp_zip_path.clone();
        move || build_zip_file(&temp_zip_path, &paths)
    })
    .await;

    match zip_result {
        Ok(Ok(())) => {}
        _ => {
            let _ = std::fs::remove_file(&temp_zip_path);
            return Ok(warp::reply::with_status(
                "Failed to create ZIP archive",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
            .into_response());
        }
    }

    let file = match fs::File::open(&temp_zip_path).await {
        Ok(file) => file,
        Err(_) => {
            let _ = std::fs::remove_file(&temp_zip_path);
            return Ok(warp::reply::with_status(
                "Failed to read ZIP archive",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
            .into_response());
        }
    };

    let file_size = match fs::metadata(&temp_zip_path).await {
        Ok(metadata) => metadata.len(),
        Err(_) => {
            let _ = std::fs::remove_file(&temp_zip_path);
            return Ok(warp::reply::with_status(
                "Failed to read ZIP archive metadata",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
            .into_response());
        }
    };

    // On Unix this unlinks the temp file while keeping the open handle readable.
    // On other platforms, removal may fail; we ignore that and keep serving.
    let _ = std::fs::remove_file(&temp_zip_path);

    let stream = ReaderStream::new(file);
    let body = Body::wrap_stream(stream);

    let disposition = "attachment; filename=\"download.zip\"";

    Ok(warp::http::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/zip")
        .header("content-disposition", disposition)
        .header("content-length", file_size.to_string())
        .body(body)
        .unwrap())
}

fn create_temp_zip_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("mediabrowser-{}-{}.zip", pid, nanos))
}

fn build_zip_file(zip_path: &Path, paths: &[String]) -> std::io::Result<()> {
    let file = File::create(zip_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for path_str in paths {
        let file_path = Path::new(path_str);

        if !file_path.starts_with(DATA_DIR) {
            continue;
        }

        let metadata = match std::fs::metadata(file_path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        if metadata.is_file() {
            let filename = file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");
            add_file_to_zip(&mut zip, file_path, filename, options)?;
        } else if metadata.is_dir() {
            add_directory_to_zip(&mut zip, file_path, file_path, options)?;
        }
    }

    if zip.finish().is_err() {
        return Err(std::io::Error::other("failed to finalize zip"));
    }
    Ok(())
}

fn add_file_to_zip(
    zip: &mut ZipWriter<File>,
    source_path: &Path,
    zip_entry_name: &str,
    options: SimpleFileOptions,
) -> std::io::Result<()> {
    let file = File::open(source_path)?;
    let mut reader = BufReader::new(file);

    if zip.start_file(zip_entry_name, options).is_err() {
        return Ok(());
    }
    std::io::copy(&mut reader, zip)?;
    Ok(())
}

fn add_directory_to_zip(
    zip: &mut ZipWriter<File>,
    dir_path: &Path,
    base_path: &Path,
    options: SimpleFileOptions,
) -> std::io::Result<()> {
    let entries = std::fs::read_dir(dir_path)?;

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let path = entry.path();
        let metadata = match std::fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        let relative_path = path
            .strip_prefix(base_path)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        if metadata.is_file() {
            let _ = add_file_to_zip(zip, &path, &relative_path, options);
        } else if metadata.is_dir() {
            let dir_name = format!("{}/", relative_path);
            let _ = zip.add_directory(&dir_name, options);
            add_directory_to_zip(zip, &path, base_path, options)?;
        }
    }

    Ok(())
}
