use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FileQuery {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct MoveQuery {
    pub from: String,
    pub to: String,
}

pub const DATA_DIR: &str = "/data";
