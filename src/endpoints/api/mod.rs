mod cp;
mod download;
mod download_bulk;
mod find;
mod mkdir;
mod mv;
mod rm;
mod upload;
mod write;

use crate::response::Response;
use crate::{field, read_form, require, Form};
use cp::handle_cp;
use download::handle_download;
use download_bulk::handle_downloads;
use find::handle_find;
use hyper::body::Incoming;
use hyper::http::request::Parts;
use hyper::http::Method;
use mkdir::handle_mkdir;
use mv::handle_mv;
use rm::handle_rm;
use upload::handle_upload;
use write::handle_write;

const SMALL_FORM_LIMIT: usize = 64 * 1024;
const DOWNLOADS_FORM_LIMIT: usize = 1024 * 1024;
const WRITE_FORM_LIMIT: usize = 16 * 1024 * 1024;

pub async fn route(parts: &Parts, body: Incoming) -> Option<Result<Response, Response>> {
    let path = parts.uri.path();

    if let Some(tail) = path.strip_prefix("/api/download/") {
        if parts.method == Method::GET {
            return Some(Ok(handle_download(tail).await));
        }
    }

    match (&parts.method, path) {
        (&Method::GET, "/api/find") => Some(Ok(find_route(parts).await)),
        (&Method::POST, "/api/downloads") => Some(downloads_route(body).await),
        (&Method::POST, "/api/upload") => Some(Ok(upload_route(parts, body).await)),
        (&Method::POST, "/api/rm") => Some(rm_route(body).await),
        (&Method::POST, "/api/mkdir") => Some(mkdir_route(body).await),
        (&Method::POST, "/api/write") => Some(write_route(body).await),
        (&Method::POST, "/api/mv") => Some(mv_route(body).await),
        (&Method::POST, "/api/cp") => Some(cp_route(body).await),
        _ => None,
    }
}

async fn find_route(parts: &Parts) -> Response {
    let form = crate::parse_form(parts.uri.query().unwrap_or_default().as_bytes());
    handle_find(field(&form, "path"), field(&form, "query")).await
}

async fn downloads_route(body: Incoming) -> Result<Response, Response> {
    let form: Form = read_form(body, DOWNLOADS_FORM_LIMIT).await?;
    Ok(handle_downloads(form).await)
}

async fn upload_route(parts: &Parts, body: Incoming) -> Response {
    let content_type = parts
        .headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    handle_upload(content_type, body).await
}

async fn rm_route(body: Incoming) -> Result<Response, Response> {
    let form = read_form(body, SMALL_FORM_LIMIT).await?;
    Ok(handle_rm(require(&form, "path")?).await)
}

async fn mkdir_route(body: Incoming) -> Result<Response, Response> {
    let form = read_form(body, SMALL_FORM_LIMIT).await?;
    Ok(handle_mkdir(require(&form, "path")?).await)
}

async fn write_route(body: Incoming) -> Result<Response, Response> {
    let form = read_form(body, WRITE_FORM_LIMIT).await?;
    Ok(handle_write(require(&form, "path")?, require(&form, "content")?).await)
}

async fn mv_route(body: Incoming) -> Result<Response, Response> {
    let form = read_form(body, SMALL_FORM_LIMIT).await?;
    Ok(handle_mv(require(&form, "from")?, require(&form, "to")?).await)
}

async fn cp_route(body: Incoming) -> Result<Response, Response> {
    let form = read_form(body, SMALL_FORM_LIMIT).await?;
    Ok(handle_cp(require(&form, "from")?, require(&form, "to")?).await)
}
