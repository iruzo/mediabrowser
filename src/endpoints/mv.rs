use crate::types::data_path;
use serde::Deserialize;
use std::convert::Infallible;
use tokio::fs;
use warp::http::StatusCode;

#[derive(Deserialize)]
pub struct MvItem {
    pub from: String,
    pub to: String,
}

pub async fn handle_mv(items: Vec<MvItem>) -> Result<impl warp::Reply, Infallible> {
    if items.is_empty() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&"No files specified"),
            StatusCode::BAD_REQUEST,
        ));
    }

    for item in items {
        let (Some(from_path), Some(to_path)) = (data_path(&item.from), data_path(&item.to)) else {
            return Ok(warp::reply::with_status(
                warp::reply::json(&"Access denied"),
                StatusCode::FORBIDDEN,
            ));
        };

        if fs::rename(&from_path, &to_path).await.is_err() {
            return Ok(warp::reply::with_status(
                warp::reply::json(&"Failed to move"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    }

    Ok(warp::reply::with_status(
        warp::reply::json(&"Moved successfully"),
        StatusCode::OK,
    ))
}
