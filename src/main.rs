use warp::Filter;

mod endpoints;
mod types;

use endpoints::{
    handle_download, handle_download_multiple,
    handle_upload, handle_delete, handle_mkdir, handle_serve
};
use types::{ListQuery, FileQuery, DATA_DIR};
use endpoints::download_multiple::DownloadMultipleQuery;

const PORT: u16 = 30003;

#[tokio::main]
async fn main() {
    let ui_index = warp::path("ui")
        .and(warp::path::end())
        .map(|| warp::reply::html(include_str!("../static/index.html")));

    let ui_css = warp::path("ui")
        .and(warp::path("styles.css"))
        .map(|| {
            warp::reply::with_header(
                include_str!("../static/styles.css"),
                "content-type",
                "text/css"
            )
        });

    let ui_js = warp::path("ui")
        .and(warp::path("script.js"))
        .map(|| {
            warp::reply::with_header(
                include_str!("../static/script.js"),
                "content-type",
                "application/javascript"
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

    let httpd_serve = warp::path::tail()
        .and_then(handle_serve);

    let routes = ui_index
        .or(ui_css)
        .or(ui_js)
        .or(api_download)
        .or(api_download_multiple)
        .or(api_upload)
        .or(api_delete)
        .or(api_mkdir)
        .or(ui_fallback)
        .or(httpd_serve);

    println!("Server starting on http://0.0.0.0:{}", PORT);
    println!("UI available at: http://0.0.0.0:{}/ui", PORT);
    println!("Serving files from: {}", DATA_DIR);

    warp::serve(routes)
        .run(([0, 0, 0, 0], PORT))
        .await;
}
