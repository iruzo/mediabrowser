use warp::Filter;

mod endpoints;
mod types;

use endpoints::download_multiple::DownloadMultipleQuery;
use endpoints::{
    handle_delete, handle_download, handle_download_multiple, handle_mkdir, handle_save,
    handle_serve, handle_upload,
};
use types::{FileQuery, ListQuery, DATA_DIR};

const PORT: u16 = 30003;

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
    let ui_index = warp::path("ui")
        .and(warp::path::end())
        .map(|| warp::reply::html(include_str!("../static/index.html")));

    let ui_css_base = warp::path("ui").and(warp::path("css")).and(warp::path("base.css")).map(|| {
        warp::reply::with_header(
            include_str!("../static/css/base.css"),
            "content-type",
            "text/css",
        )
    });

    let ui_css_layout = warp::path("ui").and(warp::path("css")).and(warp::path("layout.css")).map(|| {
        warp::reply::with_header(
            include_str!("../static/css/layout.css"),
            "content-type",
            "text/css",
        )
    });

    let ui_css_components = warp::path("ui").and(warp::path("css")).and(warp::path("components.css")).map(|| {
        warp::reply::with_header(
            include_str!("../static/css/components.css"),
            "content-type",
            "text/css",
        )
    });

    let ui_css_viewer = warp::path("ui").and(warp::path("css")).and(warp::path("viewer.css")).map(|| {
        warp::reply::with_header(
            include_str!("../static/css/viewer.css"),
            "content-type",
            "text/css",
        )
    });

    let ui_css_responsive = warp::path("ui").and(warp::path("css")).and(warp::path("responsive.css")).map(|| {
        warp::reply::with_header(
            include_str!("../static/css/responsive.css"),
            "content-type",
            "text/css",
        )
    });

    let ui_js_state = warp::path("ui").and(warp::path("js")).and(warp::path("state.js")).map(|| {
        warp::reply::with_header(
            include_str!("../static/js/state.js"),
            "content-type",
            "application/javascript",
        )
    });

    let ui_js_utils = warp::path("ui").and(warp::path("js")).and(warp::path("utils.js")).map(|| {
        warp::reply::with_header(
            include_str!("../static/js/utils.js"),
            "content-type",
            "application/javascript",
        )
    });

    let ui_js_navigation = warp::path("ui").and(warp::path("js")).and(warp::path("navigation.js")).map(|| {
        warp::reply::with_header(
            include_str!("../static/js/navigation.js"),
            "content-type",
            "application/javascript",
        )
    });

    let ui_js_gallery = warp::path("ui").and(warp::path("js")).and(warp::path("gallery.js")).map(|| {
        warp::reply::with_header(
            include_str!("../static/js/gallery.js"),
            "content-type",
            "application/javascript",
        )
    });

    let ui_js_selection = warp::path("ui").and(warp::path("js")).and(warp::path("selection.js")).map(|| {
        warp::reply::with_header(
            include_str!("../static/js/selection.js"),
            "content-type",
            "application/javascript",
        )
    });

    let ui_js_media_viewer = warp::path("ui").and(warp::path("js")).and(warp::path("media-viewer.js")).map(|| {
        warp::reply::with_header(
            include_str!("../static/js/media-viewer.js"),
            "content-type",
            "application/javascript",
        )
    });

    let ui_js_media_controls = warp::path("ui").and(warp::path("js")).and(warp::path("media-controls.js")).map(|| {
        warp::reply::with_header(
            include_str!("../static/js/media-controls.js"),
            "content-type",
            "application/javascript",
        )
    });

    let ui_js_file_ops = warp::path("ui").and(warp::path("js")).and(warp::path("file-ops.js")).map(|| {
        warp::reply::with_header(
            include_str!("../static/js/file-ops.js"),
            "content-type",
            "application/javascript",
        )
    });

    let ui_js_drawer = warp::path("ui").and(warp::path("js")).and(warp::path("drawer.js")).map(|| {
        warp::reply::with_header(
            include_str!("../static/js/drawer.js"),
            "content-type",
            "application/javascript",
        )
    });

    let ui_js_main = warp::path("ui").and(warp::path("js")).and(warp::path("main.js")).map(|| {
        warp::reply::with_header(
            include_str!("../static/js/main.js"),
            "content-type",
            "application/javascript",
        )
    });

    let ui_fallback = warp::path("ui")
        .and(warp::path::tail())
        .map(|_tail: warp::path::Tail| warp::reply::html(include_str!("../static/index.html")));

    let api_download = warp::path("api")
        .and(warp::path("download"))
        .and(warp::get())
        .and(warp::query::<FileQuery>())
        .and_then(handle_download);

    let api_download_multiple = warp::path("api")
        .and(warp::path("download-multiple"))
        .and(warp::get())
        .and(warp::query::<DownloadMultipleQuery>())
        .and_then(handle_download_multiple);

    let api_upload = warp::path("api")
        .and(warp::path("upload"))
        .and(warp::post())
        .and(warp::query::<ListQuery>())
        .and(warp::multipart::form().max_length(1024 * 1024 * 1024 * 256)) // 256GB limit
        .and_then(handle_upload);

    let api_delete = warp::path("api")
        .and(warp::path("delete"))
        .and(warp::delete())
        .and(warp::query::<FileQuery>())
        .and_then(handle_delete);

    let api_mkdir = warp::path("api")
        .and(warp::path("mkdir"))
        .and(warp::post())
        .and(warp::query::<FileQuery>())
        .and_then(handle_mkdir);

    let api_save = warp::path("api")
        .and(warp::path("save"))
        .and(warp::post())
        .and(warp::query::<FileQuery>())
        .and(warp::body::bytes())
        .and_then(handle_save);

    let httpd_serve = warp::path::tail()
        .and(warp::header::headers_cloned())
        .and_then(handle_serve);

    let routes = ui_index
        .or(ui_css_base)
        .or(ui_css_layout)
        .or(ui_css_components)
        .or(ui_css_viewer)
        .or(ui_css_responsive)
        .or(ui_js_state)
        .or(ui_js_utils)
        .or(ui_js_navigation)
        .or(ui_js_gallery)
        .or(ui_js_selection)
        .or(ui_js_media_viewer)
        .or(ui_js_media_controls)
        .or(ui_js_file_ops)
        .or(ui_js_drawer)
        .or(ui_js_main)
        .or(api_download)
        .or(api_download_multiple)
        .or(api_upload)
        .or(api_delete)
        .or(api_mkdir)
        .or(api_save)
        .or(ui_fallback)
        .or(httpd_serve);

    println!("Server starting on http://0.0.0.0:{}", PORT);
    println!("UI available at: http://0.0.0.0:{}/ui", PORT);
    println!("Serving files from: {}", DATA_DIR);

    warp::serve(routes)
        .bind_with_graceful_shutdown(([0, 0, 0, 0], PORT), shutdown_signal())
        .1
        .await;
}
