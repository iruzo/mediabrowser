use warp::Filter;

mod endpoints;
mod types;

use endpoints::{
    handle_list, handle_file, handle_download, handle_upload,
    handle_delete, handle_mkdir, handle_save
};
use types::{ListQuery, FileQuery, DATA_DIR};

const PORT: u16 = 8080;

#[tokio::main]
async fn main() {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type", "accept", "authorization", "x-requested-with"])
        .allow_methods(vec!["GET", "POST", "DELETE", "OPTIONS"]);

    let static_files = warp::path::end()
        .map(|| warp::reply::html(include_str!("../static/index.html")));

    let api_list = warp::path("api")
        .and(warp::path("list"))
        .and(warp::get())
        .and(warp::query::<ListQuery>())
        .and_then(handle_list);

    let api_file = warp::path("api")
        .and(warp::path("file"))
        .and(warp::get().or(warp::head()).unify())
        .and(warp::query::<FileQuery>())
        .and_then(handle_file);

    let api_download = warp::path("api")
        .and(warp::path("download"))
        .and(warp::get())
        .and(warp::query::<FileQuery>())
        .and_then(handle_download);

    let api_upload = warp::path("api")
        .and(warp::path("upload"))
        .and(warp::post())
        .and(warp::query::<ListQuery>())
        .and(warp::body::content_length_limit(1024 * 1024 * 100)) // 100MB limit
        .and(warp::multipart::form().max_length(1024 * 1024 * 100))
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
        .and(warp::body::content_length_limit(1024 * 1024 * 10)) // 10MB limit for text files
        .and(warp::body::bytes())
        .and_then(handle_save);

    let routes = static_files
        .or(api_list)
        .or(api_file)
        .or(api_download)
        .or(api_upload)
        .or(api_delete)
        .or(api_mkdir)
        .or(api_save)
        .with(cors);

    println!("File manager server starting on http://0.0.0.0:{}", PORT);
    println!("Serving files from: {}", DATA_DIR);

    warp::serve(routes)
        .run(([0, 0, 0, 0], PORT))
        .await;
}

