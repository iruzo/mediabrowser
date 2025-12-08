use crate::types::DATA_DIR;
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use std::convert::Infallible;
use std::io::{Cursor, Write};
use std::path::Path;
use tokio::fs;
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

    let mut zip_buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut zip_buffer);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for path_str in paths {
        let file_path = Path::new(&path_str);

        if !file_path.starts_with(DATA_DIR) {
            continue;
        }

        if let Ok(metadata) = fs::metadata(&file_path).await {
            if metadata.is_file() {
                if let Ok(contents) = fs::read(&file_path).await {
                    let filename = file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("file");

                    if zip.start_file(filename, options).is_ok() {
                        let _ = zip.write_all(&contents);
                    }
                }
            } else if metadata.is_dir() {
                if let Err(_) = add_directory_to_zip(&mut zip, file_path, file_path, options).await
                {
                    continue;
                }
            }
        }
    }

    if let Err(_) = zip.finish() {
        return Ok(warp::reply::with_status(
            "Failed to create ZIP archive",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
        .into_response());
    }

    let zip_data = zip_buffer.into_inner();
    let disposition = "attachment; filename=\"download.zip\"";

    Ok(warp::reply::with_header(
        warp::reply::with_header(zip_data, "content-type", "application/zip"),
        "content-disposition",
        disposition,
    )
    .into_response())
}

fn add_directory_to_zip<'a>(
    zip: &'a mut ZipWriter<&mut Cursor<Vec<u8>>>,
    dir_path: &'a Path,
    base_path: &'a Path,
    options: SimpleFileOptions,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), std::io::Error>> + Send + 'a>> {
    Box::pin(async move {
        let mut entries = fs::read_dir(dir_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = fs::metadata(&path).await?;

            let relative_path = path
                .strip_prefix(base_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            if metadata.is_file() {
                if let Ok(contents) = fs::read(&path).await {
                    if zip.start_file(&relative_path, options).is_ok() {
                        let _ = zip.write_all(&contents);
                    }
                }
            } else if metadata.is_dir() {
                let dir_name = format!("{}/", relative_path);
                let _ = zip.add_directory(&dir_name, options);
                add_directory_to_zip(zip, &path, base_path, options).await?;
            }
        }

        Ok(())
    })
}
