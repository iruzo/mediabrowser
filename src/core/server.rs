use crate::endpoints::{self, handle_file_server, handle_grid, handle_ui, handle_ui_paths};
use crate::response::{self, Response};
use crate::types::data_dir;
use bytes::Bytes;
use http_body::Body as HttpBody;
use hyper::body::Incoming;
use hyper::http::{Method, Request, StatusCode};
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use hyper_util::server::graceful::GracefulShutdown;
use std::convert::Infallible;
use std::future::{poll_fn, Future};
use std::net::Ipv4Addr;
use std::pin::{pin, Pin};
use std::task::Poll;
use tokio::net::TcpListener;

const PORT: u16 = 30003;
const BIND_ADDR: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const UI_FORM_LIMIT: usize = 16 * 1024 * 1024;

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

pub(crate) type Form = Vec<(String, String)>;

pub(crate) fn parse_form(bytes: &[u8]) -> Form {
    bytes
        .split(|&b| b == b'&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = match pair.iter().position(|&b| b == b'=') {
                Some(i) => (&pair[..i], &pair[i + 1..]),
                None => (pair, &pair[pair.len()..]),
            };
            (decode_form_part(key), decode_form_part(value))
        })
        .collect()
}

fn decode_form_part(bytes: &[u8]) -> String {
    let plus_decoded: Vec<u8> = bytes
        .iter()
        .map(|&b| if b == b'+' { b' ' } else { b })
        .collect();
    percent_encoding::percent_decode(&plus_decoded)
        .decode_utf8_lossy()
        .into_owned()
}

pub(crate) fn field<'a>(form: &'a Form, name: &str) -> Option<&'a str> {
    form.iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

pub(crate) fn require<'a>(form: &'a Form, name: &str) -> Result<&'a str, Response> {
    field(form, name).ok_or_else(|| {
        response::text(
            StatusCode::BAD_REQUEST,
            format!("invalid form: missing field `{name}`"),
        )
    })
}

pub(crate) async fn read_form(body: Incoming, limit: usize) -> Result<Form, Response> {
    let bytes = read_body(body, limit).await?;
    Ok(parse_form(&bytes))
}

async fn route(request: Request<Incoming>) -> Result<Response, Response> {
    let (parts, body) = request.into_parts();
    let path = parts.uri.path();

    if parts.method == Method::POST && path == "/ui" {
        let form = read_form(body, UI_FORM_LIMIT).await?;
        let paths = form
            .into_iter()
            .filter_map(|(name, path)| (name == "path").then_some(path))
            .collect();
        return Ok(handle_ui_paths(paths).await);
    }

    if let Some(result) = endpoints::api::route(&parts, body).await {
        return result;
    }

    if let Some(tail) = path.strip_prefix("/ui/") {
        if parts.method == Method::GET {
            return Ok(handle_ui(tail).await);
        }
    }

    match (&parts.method, path) {
        (&Method::GET, "/ui") => Ok(handle_ui("").await),
        (&Method::GET, "/grid") => {
            let form = parse_form(parts.uri.query().unwrap_or_default().as_bytes());
            Ok(handle_grid(field(&form, "path")).await)
        }
        (&Method::GET, "/favicon.ico") => Ok(response::status(StatusCode::OK)),
        _ => Ok(handle_file_server(path.trim_start_matches('/'), &parts.headers).await),
    }
}

async fn serve(request: Request<Incoming>) -> Result<Response, Infallible> {
    Ok(match route(request).await {
        Ok(response) | Err(response) => response,
    })
}

pub fn run() {
    tokio::runtime::Builder::new_multi_thread()
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
