use crate::types::{api_path, data_path};
use bytes::Bytes;
use futures_util::stream;
use serde::Deserialize;
use std::convert::Infallible;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use tar::Builder;
use tokio::sync::mpsc;
use warp::hyper::Body;
use warp::{http::StatusCode, Reply};

const STREAM_CHUNK_SIZE: usize = 64 * 1024;
const STREAM_CHANNEL_CAPACITY: usize = 4;

#[derive(Deserialize)]
pub struct DownloadBulkRequest {
    pub paths: Vec<String>,
}

pub async fn handle_downloads(
    request: DownloadBulkRequest,
) -> Result<warp::reply::Response, Infallible> {
    create_tar_response(request.paths)
}

fn create_tar_response(paths: Vec<String>) -> Result<warp::reply::Response, Infallible> {
    let mut data_paths = Vec::new();

    for path in paths {
        let path = path.trim();
        if path.is_empty() {
            continue;
        }

        let Some(full_path) = data_path(path) else {
            return Ok(
                warp::reply::with_status("Access denied", StatusCode::FORBIDDEN).into_response(),
            );
        };

        data_paths.push(full_path);
    }

    if data_paths.is_empty() {
        return Ok(
            warp::reply::with_status("No files specified", StatusCode::BAD_REQUEST).into_response(),
        );
    }

    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(STREAM_CHANNEL_CAPACITY);
    tokio::task::spawn_blocking({
        let tx = tx;
        move || {
            let result = build_tar_stream(&data_paths, tx.clone());
            if let Err(e) = result {
                let _ = tx.blocking_send(Err(e));
            }
        }
    });

    let stream = stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });
    let body = Body::wrap_stream(stream);
    let disposition = "attachment; filename=\"download.tar\"";

    Ok(warp::http::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/x-tar")
        .header("content-disposition", disposition)
        .body(body)
        .unwrap())
}

struct ChannelWriter {
    tx: mpsc::Sender<Result<Bytes, std::io::Error>>,
}

impl ChannelWriter {
    fn new(tx: mpsc::Sender<Result<Bytes, std::io::Error>>) -> Self {
        Self { tx }
    }
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut sent = 0usize;
        while sent < buf.len() {
            let end = (sent + STREAM_CHUNK_SIZE).min(buf.len());
            let chunk = Bytes::copy_from_slice(&buf[sent..end]);
            self.tx.blocking_send(Ok(chunk)).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, "stream closed")
            })?;
            sent = end;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn build_tar_stream(
    paths: &[PathBuf],
    tx: mpsc::Sender<Result<Bytes, std::io::Error>>,
) -> std::io::Result<()> {
    let writer = ChannelWriter::new(tx);
    let buffered = BufWriter::with_capacity(STREAM_CHUNK_SIZE, writer);
    let mut tar = Builder::new(buffered);

    for file_path in paths {
        let metadata = match std::fs::metadata(file_path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        let archive_path = archive_path_for(file_path);

        if metadata.is_file() {
            tar.append_path_with_name(file_path, archive_path)?;
        } else if metadata.is_dir() {
            tar.append_dir_all(archive_path, file_path)?;
        }
    }

    let mut output = tar.into_inner()?;
    output.flush()?;

    Ok(())
}

fn archive_path_for(file_path: &Path) -> PathBuf {
    PathBuf::from(api_path(file_path))
}
