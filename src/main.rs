use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::http::{Method, Request, StatusCode};
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use hyper_util::server::graceful::GracefulShutdown;
use std::convert::Infallible;
use std::net::Ipv4Addr;
use std::pin::pin;
use tokio::net::TcpListener;

mod endpoints;
mod response;
mod types;

use endpoints::cp::CpForm;
use endpoints::download_bulk::DownloadsForm;
use endpoints::find::FindQuery;
use endpoints::mkdir::MkdirForm;
use endpoints::mv::MvForm;
use endpoints::rm::RmForm;
use endpoints::write::WriteForm;
use endpoints::{
    handle_cp, handle_download, handle_downloads, handle_file_server, handle_find, handle_mkdir,
    handle_mv, handle_rm, handle_upload, handle_write,
};
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

    tokio::select! {
        _ = sigterm.recv() => {},
        _ = sigint.recv() => {},
    }

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

async fn read_form<T: serde::de::DeserializeOwned>(
    body: Incoming,
    limit: usize,
) -> Result<T, Response> {
    let bytes = read_body(body, limit).await?;

    serde_urlencoded::from_bytes(&bytes)
        .map_err(|error| response::text(StatusCode::BAD_REQUEST, format!("invalid form: {error}")))
}

async fn route(request: Request<Incoming>) -> Response {
    let (parts, body) = request.into_parts();
    let path = parts.uri.path();

    if let Some(tail) = path.strip_prefix("/api/download/") {
        if parts.method == Method::GET {
            return handle_download(tail).await;
        }
    }

    match (&parts.method, path) {
        (&Method::GET, "/api/find") => {
            match serde_urlencoded::from_str::<FindQuery>(parts.uri.query().unwrap_or_default()) {
                Ok(query) => handle_find(query).await,
                Err(error) => {
                    response::text(StatusCode::BAD_REQUEST, format!("invalid query: {error}"))
                }
            }
        }
        (&Method::POST, "/api/downloads") => {
            match read_form::<DownloadsForm>(body, DOWNLOADS_FORM_LIMIT).await {
                Ok(form) => handle_downloads(form).await,
                Err(response) => response,
            }
        }
        (&Method::POST, "/api/upload") => {
            let content_type = parts
                .headers
                .get("content-type")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();

            handle_upload(content_type, body).await
        }
        (&Method::POST, "/api/rm") => match read_form::<RmForm>(body, SMALL_FORM_LIMIT).await {
            Ok(form) => handle_rm(form).await,
            Err(response) => response,
        },
        (&Method::POST, "/api/mkdir") => {
            match read_form::<MkdirForm>(body, SMALL_FORM_LIMIT).await {
                Ok(form) => handle_mkdir(form).await,
                Err(response) => response,
            }
        }
        (&Method::POST, "/api/write") => {
            match read_form::<WriteForm>(body, WRITE_FORM_LIMIT).await {
                Ok(form) => handle_write(form).await,
                Err(response) => response,
            }
        }
        (&Method::POST, "/api/mv") => match read_form::<MvForm>(body, SMALL_FORM_LIMIT).await {
            Ok(form) => handle_mv(form).await,
            Err(response) => response,
        },
        (&Method::POST, "/api/cp") => match read_form::<CpForm>(body, SMALL_FORM_LIMIT).await {
            Ok(form) => handle_cp(form).await,
            Err(response) => response,
        },
        (&Method::GET, "/favicon.ico") => response::status(StatusCode::OK),
        _ => handle_file_server(path.trim_start_matches('/'), &parts.headers).await,
    }
}

async fn serve(request: Request<Incoming>) -> Result<Response, Infallible> {
    Ok(route(request).await)
}

#[tokio::main]
async fn main() {
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
        tokio::select! {
            accepted = listener.accept() => {
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
            _ = &mut shutdown => break,
        }
    }

    graceful.shutdown().await;
}
