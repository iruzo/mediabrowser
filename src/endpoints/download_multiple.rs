use crate::types::DATA_DIR;
use bytes::Bytes;
use futures_util::stream;
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use std::convert::Infallible;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::{mpsc, oneshot};
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
    if File::create(&temp_zip_path).is_err() {
        return Ok(warp::reply::with_status(
            "Failed to create ZIP archive",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
        .into_response());
    }

    let (writer_done_tx, writer_done_rx) = oneshot::channel::<std::io::Result<()>>();
    tokio::task::spawn_blocking({
        let paths = paths;
        let temp_zip_path = temp_zip_path.clone();
        move || {
            let result = build_zip_file(&temp_zip_path, &paths);
            let _ = writer_done_tx.send(result);
        }
    });

    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(8);
    tokio::spawn(stream_temp_zip_chunks(temp_zip_path.clone(), writer_done_rx, tx));

    let stream = stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });
    let body = Body::wrap_stream(stream);
    let disposition = "attachment; filename=\"download.zip\"";

    Ok(warp::http::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/zip")
        .header("content-disposition", disposition)
        .body(body)
        .unwrap())
}

async fn stream_temp_zip_chunks(
    temp_zip_path: PathBuf,
    mut writer_done_rx: oneshot::Receiver<std::io::Result<()>>,
    tx: mpsc::Sender<Result<Bytes, std::io::Error>>,
) {
    let mut file = match fs::File::open(&temp_zip_path).await {
        Ok(file) => file,
        Err(e) => {
            let _ = tx.send(Err(e)).await;
            let _ = std::fs::remove_file(&temp_zip_path);
            return;
        }
    };

    let mut offset = 0u64;
    let mut writer_done = false;
    let mut writer_error: Option<std::io::Error> = None;
    let mut buffer = vec![0u8; 64 * 1024];

    loop {
        if file
            .seek(std::io::SeekFrom::Start(offset))
            .await
            .is_err()
        {
            break;
        }

        match file.read(&mut buffer).await {
            Ok(0) => {
                if writer_done {
                    break;
                }

                tokio::select! {
                    done = &mut writer_done_rx => {
                        writer_done = true;
                        match done {
                            Ok(Ok(())) => {}
                            Ok(Err(e)) => writer_error = Some(e),
                            Err(_) => writer_error = Some(std::io::Error::other("zip worker stopped unexpectedly")),
                        }
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(20)) => {}
                }
            }
            Ok(n) => {
                offset += n as u64;
                let chunk = Bytes::copy_from_slice(&buffer[..n]);
                if tx.send(Ok(chunk)).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                let _ = tx.send(Err(e)).await;
                break;
            }
        }
    }

    if let Some(e) = writer_error {
        let _ = tx.send(Err(e)).await;
    }

    let _ = std::fs::remove_file(&temp_zip_path);
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
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

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
