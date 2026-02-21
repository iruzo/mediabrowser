use include_dir::{include_dir, Dir};
use mime_guess::from_path;
use std::convert::Infallible;
use warp::http::StatusCode;
use warp::hyper::Body;
use warp::Filter;
use warp::Reply;

static UI_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

fn index_response() -> warp::reply::Response {
    match UI_DIR.get_file("index.html") {
        Some(file) => warp::http::Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html; charset=utf-8")
            .body(Body::from(file.contents()))
            .unwrap(),
        None => warp::reply::with_status("Missing index.html", StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

fn file_response(path: &str) -> warp::reply::Response {
    match UI_DIR.get_file(path) {
        Some(file) => {
            let content_type = from_path(path).first_or_octet_stream().to_string();
            warp::http::Response::builder()
                .status(StatusCode::OK)
                .header("content-type", content_type)
                .body(Body::from(file.contents()))
                .unwrap()
        }
        None => index_response(),
    }
}

async fn serve_ui_path(tail: warp::path::Tail) -> Result<warp::reply::Response, Infallible> {
    let path = tail.as_str().trim_start_matches('/');
    if path.is_empty() {
        return Ok(index_response());
    }

    Ok(file_response(path))
}

pub fn ui_routes() -> warp::filters::BoxedFilter<(warp::reply::Response,)> {
    let ui_index = warp::path("ui")
        .and(warp::path::end())
        .map(index_response);

    let ui_path = warp::path("ui")
        .and(warp::path::tail())
        .and_then(serve_ui_path);

    ui_index.or(ui_path).unify().boxed()
}
