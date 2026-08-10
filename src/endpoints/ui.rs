use crate::response::{self, Response};
use hyper::http::StatusCode;
use percent_encoding::percent_decode_str;

const MAX_PATH_SIZE: usize = 4096;

pub async fn handle_ui(path: &str) -> Response {
    let Ok(path) = percent_decode_str(path).decode_utf8() else {
        return response::text(StatusCode::BAD_REQUEST, "path is not UTF-8");
    };

    if path.len() > MAX_PATH_SIZE || path.chars().any(|c| c.is_control() || c == '\\') {
        return response::text(StatusCode::BAD_REQUEST, "invalid path");
    }

    super::grid::handle_grid(Some(&path)).await
}

pub async fn handle_ui_paths(paths: Vec<String>) -> Response {
    super::grid::handle_grid_paths(paths).await
}

#[cfg(test)]
mod tests {
    use super::handle_ui;
    use hyper::http::StatusCode;

    fn run<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("failed to build runtime")
            .block_on(future)
    }

    #[test]
    fn rejects_control_characters_in_path() {
        let response = run(handle_ui("foo%00bar"));

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn rejects_paths_over_the_size_limit() {
        let response = run(handle_ui(&"a".repeat(5000)));

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
