use serde::Deserialize;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub path: Option<String>,
    pub query: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FileQuery {
    pub path: String,
}

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn data_dir() -> &'static Path {
    DATA_DIR
        .get_or_init(|| {
            std::env::var("DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/data"))
        })
        .as_path()
}

pub fn data_path(path: &str) -> Option<PathBuf> {
    let path = Path::new(path.trim_start_matches('/'));

    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }

    Some(data_dir().join(path))
}

pub fn api_path(path: &Path) -> String {
    path.strip_prefix(data_dir())
        .unwrap_or(path)
        .to_string_lossy()
        .trim_start_matches('/')
        .to_string()
}
