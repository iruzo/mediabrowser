use crate::response::{self, Response};
use crate::types::{data_dir, data_path};
use crate::Form;
use bytes::Bytes;
use futures_util::stream;
use hyper::http::StatusCode;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::io::{BufWriter, Write};
use std::path::{Component, Path, PathBuf};
use tar::Builder;
use tokio::fs;
use tokio::sync::mpsc;
use tokio_util::io::ReaderStream;

const MAX_PATHS: usize = 1024;
const MAX_PATH_SIZE: usize = 4096;
const STREAM_CHUNK_SIZE: usize = 64 * 1024;
const STREAM_CHANNEL_CAPACITY: usize = 4;

type DownloadResult<T> = Result<T, (StatusCode, String)>;

struct Source {
    path: PathBuf,
    name: PathBuf,
}

pub async fn handle_download(form: Form) -> Response {
    match download(form).await {
        Ok(response) => response,
        Err((status, message)) => response::text(status, message),
    }
}

async fn download(form: Form) -> DownloadResult<Response> {
    let sources = source_paths(form)?;
    let root = fs::canonicalize(data_dir()).await.map_err(download_error)?;

    // A lone regular file is served as-is; anything else is packed into a TAR.
    if let [source] = sources.as_slice() {
        if is_file(&source.path).await {
            return file_response(&source.path, &root).await;
        }
    }

    let sources = resolve_sources(sources, &root).await?;

    Ok(tar_response(sources))
}

async fn is_file(path: &Path) -> bool {
    fs::metadata(path)
        .await
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

async fn file_response(path: &Path, root: &Path) -> DownloadResult<Response> {
    let path = fs::canonicalize(path).await.map_err(download_error)?;

    if !path.starts_with(root) {
        return Err((
            StatusCode::FORBIDDEN,
            "path escapes data directory".to_string(),
        ));
    }

    let file = fs::File::open(&path).await.map_err(download_error)?;
    let metadata = file.metadata().await.map_err(download_error)?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download");
    let body = response::stream(ReaderStream::new(file));

    Ok(hyper::http::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/octet-stream")
        .header("content-disposition", content_disposition(filename))
        .header("content-length", metadata.len().to_string())
        .body(body)
        .unwrap())
}

fn tar_response(sources: Vec<Source>) -> Response {
    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(STREAM_CHANNEL_CAPACITY);

    tokio::task::spawn_blocking(move || {
        let result = build_tar_stream(&sources, tx.clone());
        if let Err(error) = result {
            let _ = tx.blocking_send(Err(error));
        }
    });

    let stream = stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });
    let body = response::stream(stream);

    hyper::http::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/x-tar")
        .header(
            "content-disposition",
            "attachment; filename=\"download.tar\"",
        )
        .body(body)
        .unwrap()
}

fn source_paths(form: Form) -> DownloadResult<Vec<Source>> {
    let mut sources = Vec::new();

    for (field, path) in form {
        if field != "path" {
            continue;
        }

        let path = path.trim();
        if path.is_empty() {
            continue;
        }
        if sources.len() == MAX_PATHS {
            return Err((
                StatusCode::BAD_REQUEST,
                "too many paths selected".to_string(),
            ));
        }

        let source = source_path(path)?;
        if sources.iter().any(|item: &Source| item.name == source.name) {
            return Err((StatusCode::BAD_REQUEST, "duplicate path".to_string()));
        }

        sources.push(source);
    }

    if sources.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "at least one path is required".to_string(),
        ));
    }

    Ok(sources)
}

fn source_path(path: &str) -> DownloadResult<Source> {
    if path.len() > MAX_PATH_SIZE || path.chars().any(|c| c.is_control() || matches!(c, '\\')) {
        return Err((StatusCode::BAD_REQUEST, "invalid path".to_string()));
    }

    let full_path =
        data_path(path).ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))?;
    let mut name = PathBuf::new();

    for component in Path::new(path.trim_start_matches('/')).components() {
        match component {
            Component::Normal(component) => name.push(component),
            Component::CurDir => {}
            _ => return Err((StatusCode::BAD_REQUEST, "invalid path".to_string())),
        }
    }

    Ok(Source {
        path: full_path,
        name,
    })
}

async fn resolve_sources(sources: Vec<Source>, root: &Path) -> DownloadResult<Vec<Source>> {
    let mut resolved = Vec::with_capacity(sources.len());

    for source in sources {
        let path = if source.path.as_path() == data_dir() {
            root.to_path_buf()
        } else {
            contained_path(source.path, root).await?
        };
        let metadata = fs::symlink_metadata(&path).await.map_err(download_error)?;
        let file_type = metadata.file_type();

        if !file_type.is_file() && !file_type.is_dir() && !file_type.is_symlink() {
            return Err((
                StatusCode::BAD_REQUEST,
                "path is not a file, directory, or symbolic link".to_string(),
            ));
        }

        resolved.push(Source {
            path,
            name: source.name,
        });
    }

    Ok(resolved)
}

async fn contained_path(path: PathBuf, root: &Path) -> DownloadResult<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))?;
    let parent = fs::canonicalize(parent).await.map_err(download_error)?;

    if !parent.starts_with(root) {
        return Err((
            StatusCode::FORBIDDEN,
            "path escapes data directory".to_string(),
        ));
    }

    let name = path
        .file_name()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "invalid path".to_string()))?;

    Ok(parent.join(name))
}

struct ChannelWriter {
    tx: mpsc::Sender<Result<Bytes, std::io::Error>>,
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut sent = 0;

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
    sources: &[Source],
    tx: mpsc::Sender<Result<Bytes, std::io::Error>>,
) -> std::io::Result<()> {
    let writer = ChannelWriter { tx };
    let buffered = BufWriter::with_capacity(STREAM_CHUNK_SIZE, writer);
    let mut tar = Builder::new(buffered);
    tar.follow_symlinks(false);

    for source in sources {
        let metadata = std::fs::symlink_metadata(&source.path)?;

        if metadata.is_dir() {
            tar.append_dir_all(&source.name, &source.path)?;
        } else {
            tar.append_path_with_name(&source.path, &source.name)?;
        }
    }

    let mut output = tar.into_inner()?;
    output.flush()
}

fn content_disposition(filename: &str) -> String {
    let filename = utf8_percent_encode(filename, NON_ALPHANUMERIC);
    format!("attachment; filename=\"download\"; filename*=UTF-8''{filename}")
}

fn download_error(error: std::io::Error) -> (StatusCode, String) {
    if error.kind() == std::io::ErrorKind::NotFound {
        (StatusCode::NOT_FOUND, "path not found".to_string())
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create download: {error}"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{content_disposition, source_path, source_paths, MAX_PATH_SIZE};
    use crate::Form;
    use hyper::http::StatusCode;

    #[test]
    fn accepts_repeated_path_fields() {
        let form: Form = vec![
            ("path".to_string(), "first.txt".to_string()),
            ("ignored".to_string(), "value".to_string()),
            ("path".to_string(), "folder/second.txt".to_string()),
        ];
        let sources = source_paths(form).unwrap();

        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].name.to_string_lossy(), "first.txt");
        assert_eq!(sources[1].name.to_string_lossy(), "folder/second.txt");
    }

    #[test]
    fn requires_at_least_one_path() {
        for form in [
            vec![],
            vec![("ignored".to_string(), "value".to_string())],
            vec![("path".to_string(), "  ".to_string())],
        ] {
            assert_eq!(
                source_paths(form).err(),
                Some((
                    StatusCode::BAD_REQUEST,
                    "at least one path is required".to_string()
                ))
            );
        }
    }

    #[test]
    fn rejects_duplicate_paths() {
        let form: Form = vec![
            ("path".to_string(), "file.txt".to_string()),
            ("path".to_string(), "/file.txt".to_string()),
        ];

        assert_eq!(
            source_paths(form).err(),
            Some((StatusCode::BAD_REQUEST, "duplicate path".to_string()))
        );
    }

    #[test]
    fn validates_download_paths() {
        for path in ["file.txt", "/folder/file.txt", "árbol/東京.txt", "/"] {
            assert!(source_path(path).is_ok());
        }

        for path in ["..", "../file", "folder/../file", "a\\b", "a\nb", "a\0b"] {
            assert_eq!(
                source_path(path).err(),
                Some((StatusCode::BAD_REQUEST, "invalid path".to_string()))
            );
        }

        assert_eq!(
            source_path(&"a".repeat(MAX_PATH_SIZE + 1)).err(),
            Some((StatusCode::BAD_REQUEST, "invalid path".to_string()))
        );
    }

    #[test]
    fn encodes_download_filename() {
        let disposition = content_disposition("á\".txt");

        assert_eq!(
            disposition,
            "attachment; filename=\"download\"; filename*=UTF-8''%C3%A1%22%2Etxt"
        );
    }
}
