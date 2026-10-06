#[cfg(feature = "api")]
use crate::endpoints;
#[cfg(feature = "httpd")]
use crate::endpoints::handle_file_server;
#[cfg(feature = "ui")]
use crate::endpoints::handle_ui_path;
use crate::response::{self, Response};
use crate::types::data_dir;
use crate::Access;
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
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::pin;
#[cfg(feature = "api")]
use std::pin::Pin;
#[cfg(feature = "auth")]
use std::sync::Arc;
use std::task::Poll;
#[cfg(feature = "https")]
use std::time::Duration;
use tokio::net::TcpListener;

const PORT: u16 = 30003;
const BIND_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

fn get_bind_addr() -> IpAddr {
    match std::env::var("BIND_ADDR") {
        Ok(value) => match value.parse::<IpAddr>() {
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

async fn route(request: Request<Incoming>, access: &Access) -> Response {
    let (parts, body) = request.into_parts();
    let path = parts.uri.path();

    #[cfg(feature = "api")]
    if let Some(response) = endpoints::api::route(&parts, body, access).await {
        return response;
    }

    #[cfg(not(feature = "api"))]
    drop(body);

    #[cfg(feature = "ui")]
    if parts.method == Method::GET && (path == "/ui" || path.starts_with("/ui/")) {
        let path = percent_encoding::percent_decode_str(
            path.strip_prefix("/ui")
                .unwrap_or("")
                .trim_start_matches('/'),
        )
        .decode_utf8_lossy();
        if let Err(response) = access.check(&path, false) {
            return response;
        }
        return handle_ui_path(&parts.uri, &parts.headers).await;
    }

    if parts.method == Method::GET && path == "/favicon.ico" {
        return response::status(StatusCode::OK);
    }

    #[cfg(feature = "httpd")]
    return handle_file_server(path.trim_start_matches('/'), &parts.headers, access).await;

    #[cfg(not(feature = "httpd"))]
    {
        let _ = access;
        response::text(StatusCode::NOT_FOUND, "Not found")
    }
}

async fn dispatch(
    request: Request<Incoming>,
    #[cfg(feature = "auth")] auth: &super::auth::Auth,
    #[cfg(feature = "cors")] cors: bool,
) -> Response {
    #[cfg(feature = "auth")]
    let access = {
        if request.uri().path() == "/api/login" {
            return auth.login(request).await;
        }
        if request.uri().path() == "/api/logout" {
            return auth.logout(&request);
        }
        let Some(access) = auth.session(request.headers()).await else {
            return super::auth::redirect("/api/login");
        };
        if request.method() == Method::POST && !super::auth::same_origin(request.headers()) {
            return response::text(StatusCode::FORBIDDEN, "Access denied");
        }
        if request.uri().path() == "/api/password" {
            return auth.password(request, &access).await;
        }
        access
    };
    #[cfg(not(feature = "auth"))]
    let access = Access::default();
    #[cfg(feature = "cors")]
    if cors {
        if let Some(response) = super::cors::preflight(&request) {
            return response;
        }
    }
    route(request, &access).await
}

async fn serve(
    request: Request<Incoming>,
    #[cfg(feature = "auth")] auth: Arc<super::auth::Auth>,
    #[cfg(feature = "cors")] origin: Option<hyper::http::HeaderValue>,
) -> Result<Response, Infallible> {
    #[allow(unused_mut)]
    let mut response = dispatch(
        request,
        #[cfg(feature = "auth")]
        &auth,
        #[cfg(feature = "cors")]
        origin.is_some(),
    )
    .await;
    #[cfg(feature = "cors")]
    if let Some(origin) = origin {
        response
            .headers_mut()
            .insert(hyper::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    }
    #[cfg(feature = "auth")]
    response.headers_mut().insert(
        "cache-control",
        hyper::http::HeaderValue::from_static("no-store"),
    );
    Ok(response)
}

pub fn run() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build runtime")
        .block_on(run_async());
}

async fn run_async() {
    #[cfg(feature = "auth")]
    let auth = Arc::new(super::auth::Auth::new());
    #[cfg(feature = "firewall")]
    let firewall = match super::firewall::Firewall::from_env() {
        Ok(firewall) => firewall,
        Err(error) => {
            eprintln!("Failed to configure firewall: {error}");
            std::process::exit(1);
        }
    };
    #[cfg(feature = "cors")]
    let origin = match super::cors::origin() {
        Ok(origin) => origin,
        Err(error) => {
            eprintln!("Failed to configure CORS: {error}");
            std::process::exit(1);
        }
    };
    #[cfg(feature = "https")]
    let tls = match super::tls::acceptor() {
        Ok(tls) => tls,
        Err(error) => {
            eprintln!("Failed to configure HTTPS: {error}");
            std::process::exit(1);
        }
    };
    let address = SocketAddr::new(get_bind_addr(), get_port());
    let listener = TcpListener::bind(address)
        .await
        .expect("failed to bind server address");

    let scheme = if cfg!(feature = "https") {
        "https"
    } else {
        "http"
    };
    println!(
        "Server starting on {}://{}",
        scheme,
        listener.local_addr().unwrap()
    );
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
        let Ok((stream, peer)) = accepted else {
            continue;
        };
        #[cfg(feature = "firewall")]
        if !firewall.allows(peer.ip()) {
            continue;
        }
        #[cfg(not(feature = "firewall"))]
        let _ = peer;
        #[cfg(feature = "https")]
        let tls = tls.clone();
        #[cfg(feature = "cors")]
        let origin = origin.clone();
        let watcher = graceful.watcher();
        #[cfg(feature = "auth")]
        let auth = auth.clone();

        tokio::spawn(async move {
            #[cfg(feature = "https")]
            let stream = {
                // Bound idle handshakes so they cannot delay shutdown indefinitely.
                match tokio::time::timeout(Duration::from_secs(10), tls.accept(stream)).await {
                    Ok(Ok(stream)) => stream,
                    Ok(Err(error)) => {
                        eprintln!("TLS handshake failed: {error}");
                        return;
                    }
                    Err(_) => {
                        eprintln!("TLS handshake timed out");
                        return;
                    }
                }
            };
            let service = service_fn(move |request| {
                serve(
                    request,
                    #[cfg(feature = "auth")]
                    auth.clone(),
                    #[cfg(feature = "cors")]
                    origin.clone(),
                )
            });
            let connection = hyper::server::conn::http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service);
            let _ = watcher.watch(connection).await;
        });
    }

    graceful.shutdown().await;
}
