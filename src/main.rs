use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::http::{Method, Request, StatusCode};
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use hyper_util::server::graceful::GracefulShutdown;
use std::convert::Infallible;
use std::future::{poll_fn, Future};
use std::net::Ipv4Addr;
use std::pin::pin;
use std::task::Poll;
use tokio::net::TcpListener;

mod endpoints;
mod multipart;
mod response;
mod types;

use endpoints::{
    handle_cp, handle_download, handle_downloads, handle_find, handle_mkdir, handle_mv, handle_rm,
    handle_upload, handle_write,
};
#[cfg(grid)]
use endpoints::handle_grid;
#[cfg(httpd)]
use endpoints::handle_file_server;
#[cfg(ui)]
use endpoints::handle_ui;
use response::Response;
use types::data_dir;

const PORT: u16 = 30003;
const BIND_ADDR: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const SMALL_FORM_LIMIT: usize = 64 * 1024;
const DOWNLOADS_FORM_LIMIT: usize = 1024 * 1024;
const WRITE_FORM_LIMIT: usize = 16 * 1024 * 1024;

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

    while let Some(frame) = body.frame().await {
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

type Form = Vec<(String, String)>;

fn parse_form(bytes: &[u8]) -> Form {
    form_urlencoded::parse(bytes).into_owned().collect()
}

fn field<'a>(form: &'a Form, name: &str) -> Option<&'a str> {
    form.iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn require<'a>(form: &'a Form, name: &str) -> Result<&'a str, Response> {
    field(form, name).ok_or_else(|| {
        response::text(
            StatusCode::BAD_REQUEST,
            format!("invalid form: missing field `{name}`"),
        )
    })
}

async fn read_form(body: Incoming, limit: usize) -> Result<Form, Response> {
    let bytes = read_body(body, limit).await?;
    Ok(parse_form(&bytes))
}

async fn route(request: Request<Incoming>) -> Result<Response, Response> {
    let (parts, body) = request.into_parts();
    let path = parts.uri.path();

    if let Some(tail) = path.strip_prefix("/api/download/") {
        if parts.method == Method::GET {
            return Ok(handle_download(tail).await);
        }
    }
    #[cfg(ui)]
    if let Some(tail) = path.strip_prefix("/ui/") {
        if parts.method == Method::GET {
            return Ok(handle_ui(tail).await);
        }
    }

    match (&parts.method, path) {
        #[cfg(ui)]
        (&Method::GET, "/ui") => Ok(handle_ui("").await),
        #[cfg(grid)]
        (&Method::GET, "/grid") => {
            let form = parse_form(parts.uri.query().unwrap_or_default().as_bytes());
            Ok(handle_grid(field(&form, "path")).await)
        }
        (&Method::GET, "/api/find") => {
            let form = parse_form(parts.uri.query().unwrap_or_default().as_bytes());
            Ok(handle_find(field(&form, "path"), field(&form, "query")).await)
        }
        (&Method::POST, "/api/downloads") => {
            let form = read_form(body, DOWNLOADS_FORM_LIMIT).await?;
            Ok(handle_downloads(form).await)
        }
        (&Method::POST, "/api/upload") => {
            let content_type = parts
                .headers
                .get("content-type")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();

            Ok(handle_upload(content_type, body).await)
        }
        (&Method::POST, "/api/rm") => {
            let form = read_form(body, SMALL_FORM_LIMIT).await?;
            Ok(handle_rm(require(&form, "path")?).await)
        }
        (&Method::POST, "/api/mkdir") => {
            let form = read_form(body, SMALL_FORM_LIMIT).await?;
            Ok(handle_mkdir(require(&form, "path")?).await)
        }
        (&Method::POST, "/api/write") => {
            let form = read_form(body, WRITE_FORM_LIMIT).await?;
            Ok(handle_write(require(&form, "path")?, require(&form, "content")?).await)
        }
        (&Method::POST, "/api/mv") => {
            let form = read_form(body, SMALL_FORM_LIMIT).await?;
            Ok(handle_mv(require(&form, "from")?, require(&form, "to")?).await)
        }
        (&Method::POST, "/api/cp") => {
            let form = read_form(body, SMALL_FORM_LIMIT).await?;
            Ok(handle_cp(require(&form, "from")?, require(&form, "to")?).await)
        }
        (&Method::GET, "/favicon.ico") => Ok(response::status(StatusCode::OK)),
        #[cfg(httpd)]
        _ => Ok(handle_file_server(path.trim_start_matches('/'), &parts.headers).await),
        #[cfg(not(httpd))]
        _ => Ok(response::status(StatusCode::NOT_FOUND)),
    }
}

async fn serve(request: Request<Incoming>) -> Result<Response, Infallible> {
    Ok(match route(request).await {
        Ok(response) | Err(response) => response,
    })
}

fn main() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build runtime")
        .block_on(run());
}

async fn run() {
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
