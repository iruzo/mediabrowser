use include_dir::{include_dir, Dir};
use mime_guess::from_path;
use warp::http::StatusCode;
use warp::hyper::Body;
use warp::Filter;

static UI_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

async fn serve_ui_path(tail: warp::path::Tail) -> Result<warp::reply::Response, warp::Rejection> {
    let path = tail.as_str().trim_start_matches('/');

    match UI_DIR.get_file(path) {
        Some(file) => {
            let content_type = from_path(path).first_or_octet_stream().to_string();
            Ok(warp::http::Response::builder()
                .status(StatusCode::OK)
                .header("content-type", content_type)
                .body(Body::from(file.contents()))
                .unwrap())
        }
        None => Err(warp::reject::not_found()),
    }
}

pub fn ui_routes() -> warp::filters::BoxedFilter<(warp::reply::Response,)> {
    warp::path("ui")
        .and(warp::path::tail())
        .and_then(serve_ui_path)
        .boxed()
}
