use bytes::Bytes;
use futures_util::Stream;
use http_body::{Body as HttpBody, Frame, SizeHint};
use hyper::http::StatusCode;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

pub enum Body {
    Empty,
    Full(Option<Bytes>),
    Stream(Pin<Box<dyn Stream<Item = Result<Bytes, io::Error>> + Send + Sync>>),
}

impl HttpBody for Body {
    type Data = Bytes;
    type Error = io::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, io::Error>>> {
        match self.get_mut() {
            Body::Empty => Poll::Ready(None),
            Body::Full(data) => Poll::Ready(data.take().map(|data| Ok(Frame::data(data)))),
            Body::Stream(stream) => stream
                .as_mut()
                .poll_next(cx)
                .map(|item| item.map(|result| result.map(Frame::data))),
        }
    }

    fn is_end_stream(&self) -> bool {
        matches!(self, Body::Empty | Body::Full(None))
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Body::Empty | Body::Full(None) => SizeHint::with_exact(0),
            Body::Full(Some(data)) => SizeHint::with_exact(data.len() as u64),
            Body::Stream(_) => SizeHint::default(),
        }
    }
}

pub type Response = hyper::http::Response<Body>;

pub fn empty() -> Body {
    Body::Empty
}

pub fn full(data: impl Into<Bytes>) -> Body {
    let data = data.into();
    if data.is_empty() {
        Body::Full(None)
    } else {
        Body::Full(Some(data))
    }
}

pub fn stream<S>(stream: S) -> Body
where
    S: Stream<Item = Result<Bytes, io::Error>> + Send + Sync + 'static,
{
    Body::Stream(Box::pin(stream))
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
