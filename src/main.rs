use std::net::Ipv4Addr;
use warp::Filter;

mod endpoints;
mod types;

use endpoints::download_bulk::DownloadsForm;
use endpoints::find::FindQuery;
use endpoints::mkdir::MkdirForm;
use endpoints::mv::MvForm;
use endpoints::rm::RmForm;
use endpoints::write::WriteForm;
use endpoints::{
    handle_download, handle_downloads, handle_file_server, handle_find, handle_mkdir, handle_mv,
    handle_rm, handle_upload, handle_write, render_routes, ui_routes,
};
use types::data_dir;

const PORT: u16 = 30003;
const BIND_ADDR: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const SMALL_FORM_LIMIT: u64 = 64 * 1024;
const DOWNLOADS_FORM_LIMIT: u64 = 1024 * 1024;
const WRITE_FORM_LIMIT: u64 = 16 * 1024 * 1024;
const MAX_UPLOAD_SIZE: u64 = 256 * 1024 * 1024 * 1024;

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

#[tokio::main]
async fn main() {
    let bind_addr = get_bind_addr();
    let port = get_port();

    let api_download = warp::path("api")
        .and(warp::path("download"))
        .and(warp::get())
        .and(warp::path::tail())
        .and_then(handle_download);

    let api_downloads = warp::path("api")
        .and(warp::path("downloads"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::content_length_limit(DOWNLOADS_FORM_LIMIT))
        .and(warp::body::form::<DownloadsForm>())
        .and_then(handle_downloads);

    let api_upload = warp::path("api")
        .and(warp::path("upload"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::multipart::form().max_length(MAX_UPLOAD_SIZE))
        .and_then(handle_upload);

    let api_find = warp::path("api")
        .and(warp::path("find"))
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<FindQuery>())
        .and_then(handle_find);

    let api_rm = warp::path("api")
        .and(warp::path("rm"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::content_length_limit(SMALL_FORM_LIMIT))
        .and(warp::body::form::<RmForm>())
        .and_then(handle_rm);

    let api_mkdir = warp::path("api")
        .and(warp::path("mkdir"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::content_length_limit(SMALL_FORM_LIMIT))
        .and(warp::body::form::<MkdirForm>())
        .and_then(handle_mkdir);

    let api_write = warp::path("api")
        .and(warp::path("write"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::content_length_limit(WRITE_FORM_LIMIT))
        .and(warp::body::form::<WriteForm>())
        .and_then(handle_write);

    let api_mv = warp::path("api")
        .and(warp::path("mv"))
        .and(warp::path::end())
        .and(warp::post())
        .and(warp::body::content_length_limit(SMALL_FORM_LIMIT))
        .and(warp::body::form::<MvForm>())
        .and_then(handle_mv);

    let favicon = warp::path("favicon.ico")
        .and(warp::path::end())
        .and(warp::get())
        .map(|| "");

    let file_server = warp::path::tail()
        .and(warp::header::headers_cloned())
        .and_then(handle_file_server);

    let routes = ui_routes()
        .or(render_routes())
        .or(api_download)
        .or(api_downloads)
        .or(api_upload)
        .or(api_find)
        .or(api_rm)
        .or(api_mkdir)
        .or(api_write)
        .or(api_mv)
        .or(favicon)
        .or(file_server);

    println!("Server starting on http://{}:{}", bind_addr, port);
    println!("UI available at: http://{}:{}/ui", bind_addr, port);
    println!("Serving files from: {}", data_dir().display());

    warp::serve(routes)
        .bind_with_graceful_shutdown((bind_addr.octets(), port), shutdown_signal())
        .1
        .await;
}
