#[cfg(feature = "api")]
use crate::endpoints;
#[cfg(feature = "httpd")]
use crate::endpoints::handle_file_server;
#[cfg(feature = "ui")]
use crate::endpoints::handle_ui_path;
use crate::response::{self, Response};
use crate::types::data_dir;
#[cfg(feature = "api")]
use bytes::Bytes;
#[cfg(feature = "api")]
use http_body::Body as HttpBody;
use hyper::body::Incoming;
use hyper::http::{Method, Request, StatusCode};
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use hyper_util::server::graceful::GracefulShutdown;
use std::convert::Infallible;
use std::future::{poll_fn, Future};
use std::net::Ipv4Addr;
use std::pin::pin;
#[cfg(feature = "api")]
use std::pin::Pin;
use std::task::Poll;
use tokio::net::TcpListener;

const PORT: u16 = 30003;
const BIND_ADDR: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

fn get_bind_addr() -> Ipv4Addr {
    match std::env::var("BIND_ADDR") {
        Ok(value) => match value.parse::<Ipv4Addr>() {
            Ok(addr) => addr,
            Err(_) => {
                eprintln!("Invalid BIND_ADDR='{}', using default {}", value, BIND_ADDR);
                BIND_ADDR
            }
        },
        Err(_) => BIND_ADDR,
    }
}

fn get_port() -> u16 {
    match std::env::var("PORT") {
        Ok(value) => match value.parse::<u16>() {
            Ok(port) => port,
            Err(_) => {
                eprintln!("Invalid PORT='{}', using default {}", value, PORT);
                PORT
            }
        },
        Err(_) => PORT,
    }
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");

    poll_fn(|cx| {
        if sigterm.poll_recv(cx).is_ready() || sigint.poll_recv(cx).is_ready() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;

    println!("Shutdown signal received, stopping server gracefully...");
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    use tokio::signal;

    signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C handler");

    println!("Shutdown signal received, stopping server gracefully...");
}

#[cfg(feature = "api")]
async fn read_body(mut body: Incoming, limit: usize) -> Result<Bytes, Response> {
    let mut data = Vec::new();

    while let Some(frame) = poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await {
        let frame = frame.map_err(|error| {
            response::text(
                StatusCode::BAD_REQUEST,
                format!("Failed to read request body: {error}"),
            )
        })?;

        if let Ok(chunk) = frame.into_data() {
            if data.len() + chunk.len() > limit {
                return Err(response::text(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "request body is too large",
                ));
            }
            data.extend_from_slice(&chunk);
        }
    }

    Ok(Bytes::from(data))
}

#[cfg(feature = "api")]
pub(crate) type Form = Vec<(String, String)>;

#[cfg(feature = "api")]
fn parse_form(bytes: &[u8]) -> Form {
    bytes
        .split(|&b| b == b'&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = match pair.iter().position(|&b| b == b'=') {
                Some(i) => (&pair[..i], &pair[i + 1..]),
                _none => (pair, &pair[pair.len()..]),
            };
            (decode_form_part(key), decode_form_part(value))
        })
        .collect()
}

#[cfg(feature = "api")]
fn decode_form_part(bytes: &[u8]) -> String {
    let plus_decoded: Vec<u8> = bytes
        .iter()
        .map(|&b| if b == b'+' { b' ' } else { b })
        .collect();
    percent_encoding::percent_decode(&plus_decoded)
        .decode_utf8_lossy()
        .into_owned()
}

#[cfg(feature = "api")]
pub(crate) fn field<'a>(form: &'a Form, name: &str) -> Option<&'a str> {
    form.iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

#[cfg(feature = "api")]
pub(crate) fn require<'a>(form: &'a Form, name: &str) -> Result<&'a str, Response> {
    field(form, name).ok_or_else(|| {
        response::text(
            StatusCode::BAD_REQUEST,
            format!("invalid form: missing field `{name}`"),
        )
    })
}

#[cfg(feature = "api")]
pub(crate) async fn read_form(body: Incoming, limit: usize) -> Result<Form, Response> {
    let bytes = read_body(body, limit).await?;
    Ok(parse_form(&bytes))
}

async fn route(request: Request<Incoming>) -> Response {
    let (parts, body) = request.into_parts();
    let path = parts.uri.path();

    #[cfg(feature = "api")]
    if let Some(response) = endpoints::api::route(&parts, body).await {
        return response;
    }

    #[cfg(not(feature = "api"))]
    drop(body);

    #[cfg(feature = "ui")]
    if parts.method == Method::GET && (path == "/ui" || path.starts_with("/ui/")) {
        return handle_ui_path(&parts.uri, &parts.headers).await;
    }

    if parts.method == Method::GET && path == "/favicon.ico" {
        return response::status(StatusCode::OK);
    }

    #[cfg(feature = "httpd")]
    return handle_file_server(path.trim_start_matches('/'), &parts.headers).await;

    #[cfg(not(feature = "httpd"))]
    response::text(StatusCode::NOT_FOUND, "Not found")
}

async fn serve(request: Request<Incoming>) -> Result<Response, Infallible> {
    Ok(route(request).await)
}

pub fn run() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build runtime")
        .block_on(run_async());
}

async fn run_async() {
    let bind_addr = get_bind_addr();
    let port = get_port();
    let listener = TcpListener::bind((bind_addr, port))
        .await
        .expect("failed to bind server address");

    println!("Server starting on http://{}:{}", bind_addr, port);
    println!("Serving files from: {}", data_dir().display());

    let graceful = GracefulShutdown::new();
    let mut shutdown = pin!(shutdown_signal());

    loop {
        // Resolve to None when the shutdown signal wins over an incoming connection
        let accepted = poll_fn(|cx| {
            if shutdown.as_mut().poll(cx).is_ready() {
                return Poll::Ready(None);
            }
            listener.poll_accept(cx).map(Some)
        })
        .await;

        let Some(accepted) = accepted else {
            break;
        };
        let Ok((stream, _)) = accepted else {
            continue;
        };
        let connection = hyper::server::conn::http1::Builder::new()
            .serve_connection(TokioIo::new(stream), service_fn(serve));
        let connection = graceful.watch(connection);

        tokio::spawn(async move {
            let _ = connection.await;
        });
    }

    graceful.shutdown().await;
}
