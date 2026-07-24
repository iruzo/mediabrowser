use bytes::Bytes;
use futures_util::{Stream, TryStreamExt};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full, StreamBody};
use hyper::body::Frame;
use hyper::http::StatusCode;

pub type Body = BoxBody<Bytes, std::io::Error>;
pub type Response = hyper::http::Response<Body>;

pub fn empty() -> Body {
    Empty::new().map_err(unreachable_error).boxed()
}

pub fn full(data: impl Into<Bytes>) -> Body {
    Full::new(data.into()).map_err(unreachable_error).boxed()
}

pub fn stream<S>(stream: S) -> Body
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + Send + Sync + 'static,
{
    StreamBody::new(stream.map_ok(Frame::data)).boxed()
}

pub fn status(status: StatusCode) -> Response {
    hyper::http::Response::builder()
        .status(status)
        .body(empty())
        .expect("valid empty response")
}

pub fn text(status: StatusCode, message: impl Into<String>) -> Response {
    hyper::http::Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(full(message.into()))
        .expect("valid text response")
}

#[cfg(grid)]
pub fn html(body: impl Into<String>) -> Response {
    hyper::http::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/html; charset=utf-8")
        .body(full(body.into()))
        .expect("valid HTML response")
}

pub fn json(body: String) -> Response {
    hyper::http::Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(full(body))
        .expect("valid JSON response")
}

fn unreachable_error(error: std::convert::Infallible) -> std::io::Error {
    match error {}
}
