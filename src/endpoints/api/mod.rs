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

pub async fn route(parts: &Parts, body: Incoming, access: &crate::Access) -> Option<Response> {
    if parts.method != Method::POST {
        return None;
    }
    let action = parts.uri.path();
    if action == "/api/upload" {
        let content_type = parts
            .headers
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        return Some(handle_upload(content_type, body, access).await);
    }
    if !matches!(
        action,
        "/api/cat"
            | "/api/find"
            | "/api/download"
            | "/api/rm"
            | "/api/mkdir"
            | "/api/mv"
            | "/api/cp"
    ) {
        return None;
    }
    Some(into_response(form_route(action, parts, body, access).await))
}

fn into_response(result: Result<Response, Response>) -> Response {
    match result {
        Ok(response) | Err(response) => response,
    }
}

async fn form_route(
    action: &str,
    parts: &Parts,
    body: Incoming,
    access: &crate::Access,
) -> Result<Response, Response> {
    let limit = if action == "/api/download" {
        DOWNLOAD_FORM_LIMIT
    } else {
        SMALL_FORM_LIMIT
    };
    let form: Form = read_form(body, limit).await?;
    match action {
        "/api/mv" | "/api/cp" => {
            access.check(require(&form, "from")?, action == "/api/mv")?;
            access.check(require(&form, "to")?, true)?;
        }
        "/api/download" => {
            for (name, path) in &form {
                if name == "path" {
                    access.check(path, false)?;
                }
            }
        }
        _ => access.check(
            field(&form, "path").unwrap_or(""),
            matches!(action, "/api/rm" | "/api/mkdir"),
        )?,
    }
    Ok(match action {
        "/api/cat" => handle_cat(field(&form, "path"), &parts.headers).await,
        "/api/find" => {
            handle_find(
                field(&form, "path"),
                field(&form, "query"),
                field(&form, "type"),
                field(&form, "recursive"),
                field(&form, "metadata"),
                access,
            )
            .await
        }
        "/api/download" => handle_download(form, access).await,
        "/api/rm" => handle_rm(require(&form, "path")?).await,
        "/api/mkdir" => handle_mkdir(require(&form, "path")?).await,
        "/api/mv" => handle_mv(require(&form, "from")?, require(&form, "to")?).await,
        "/api/cp" => handle_cp(require(&form, "from")?, require(&form, "to")?, access).await,
        _ => unreachable!(),
    })
}
