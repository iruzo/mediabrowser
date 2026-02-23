use crate::types::DATA_DIR;
use bytes::Bytes;
use futures_util::stream;
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use std::convert::Infallible;
use std::fs::File;
use std::io::BufReader;
use std::io::{BufWriter, Seek, Write};
use std::path::Path;
use tokio::sync::mpsc;
use warp::hyper::Body;
use warp::{http::StatusCode, Reply};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

const STREAM_CHUNK_SIZE: usize = 64 * 1024;
const STREAM_CHANNEL_CAPACITY: usize = 4;

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

    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(STREAM_CHANNEL_CAPACITY);
    tokio::task::spawn_blocking({
        let paths = paths;
        let tx = tx;
        move || {
            let result = build_zip_stream(&paths, tx.clone());
            if let Err(e) = result {
                let _ = tx.blocking_send(Err(e));
            }
        }
    });

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

fn build_zip_stream(
    paths: &[String],
    tx: mpsc::Sender<Result<Bytes, std::io::Error>>,
) -> std::io::Result<()> {
    let writer = ChannelWriter::new(tx);
    let buffered = BufWriter::with_capacity(STREAM_CHUNK_SIZE, writer);
    let mut zip = ZipWriter::new_stream(buffered);
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

    let mut output = zip
        .finish()
        .map_err(|_| std::io::Error::other("failed to finalize zip"))?;
    output.flush()?;

    Ok(())
}

fn add_file_to_zip<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
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

fn add_directory_to_zip<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
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
