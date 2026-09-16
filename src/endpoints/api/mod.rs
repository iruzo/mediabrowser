mod cat;
mod cp;
mod download;
mod find;
mod mkdir;
mod mv;
mod rm;
mod upload;

use crate::response::Response;
use crate::{field, read_form, require, Form};
use cat::handle_cat;
use cp::handle_cp;
use download::handle_download;
use find::handle_find;
use hyper::body::Incoming;
use hyper::http::request::Parts;
use hyper::http::Method;
use mkdir::handle_mkdir;
use mv::handle_mv;
use rm::handle_rm;
use upload::handle_upload;

const SMALL_FORM_LIMIT: usize = 64 * 1024;
const DOWNLOAD_FORM_LIMIT: usize = 1024 * 1024;

pub async fn route(parts: &Parts, body: Incoming) -> Option<Response> {
    let response = match (&parts.method, parts.uri.path()) {
        (&Method::POST, "/api/cat") => into_response(cat_route(parts, body).await),
        (&Method::POST, "/api/find") => into_response(find_route(body).await),
        (&Method::POST, "/api/download") => into_response(download_route(body).await),
        (&Method::POST, "/api/upload") => upload_route(parts, body).await,
        (&Method::POST, "/api/rm") => into_response(rm_route(body).await),
        (&Method::POST, "/api/mkdir") => into_response(mkdir_route(body).await),
        (&Method::POST, "/api/mv") => into_response(mv_route(body).await),
        (&Method::POST, "/api/cp") => into_response(cp_route(body).await),
        _ => return None,
    };

    Some(response)
}

async fn cat_route(parts: &Parts, body: Incoming) -> Result<Response, Response> {
    let form = read_form(body, SMALL_FORM_LIMIT).await?;
    Ok(handle_cat(field(&form, "path"), &parts.headers).await)
}

fn into_response(result: Result<Response, Response>) -> Response {
    match result {
        Ok(response) | Err(response) => response,
    }
}

async fn find_route(body: Incoming) -> Result<Response, Response> {
    let form = read_form(body, SMALL_FORM_LIMIT).await?;
    Ok(handle_find(
        field(&form, "path"),
        field(&form, "query"),
        field(&form, "type"),
        field(&form, "recursive"),
        field(&form, "metadata"),
    )
    .await)
}

async fn download_route(body: Incoming) -> Result<Response, Response> {
    let form: Form = read_form(body, DOWNLOAD_FORM_LIMIT).await?;
    Ok(handle_download(form).await)
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

async fn mv_route(body: Incoming) -> Result<Response, Response> {
    let form = read_form(body, SMALL_FORM_LIMIT).await?;
    Ok(handle_mv(require(&form, "from")?, require(&form, "to")?).await)
}

async fn cp_route(body: Incoming) -> Result<Response, Response> {
    let form = read_form(body, SMALL_FORM_LIMIT).await?;
    Ok(handle_cp(require(&form, "from")?, require(&form, "to")?).await)
}
