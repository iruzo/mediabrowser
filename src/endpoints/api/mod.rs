#[cfg(feature = "auth")]
mod login;
#[cfg(feature = "auth")]
mod logout;
#[cfg(feature = "auth")]
mod password;

#[cfg(feature = "auth")]
pub(crate) use login::handle_login;
#[cfg(feature = "auth")]
pub(crate) use logout::handle_logout;
#[cfg(feature = "auth")]
pub(crate) use password::handle_password;

#[cfg(feature = "api")]
mod cat;
#[cfg(feature = "api")]
mod cp;
#[cfg(feature = "api")]
mod download;
#[cfg(feature = "api")]
mod find;
#[cfg(feature = "api")]
mod mkdir;
#[cfg(feature = "api")]
mod mv;
#[cfg(feature = "api")]
mod rm;
#[cfg(feature = "api")]
mod upload;

#[cfg(feature = "api")]
use crate::response::Response;
#[cfg(feature = "api")]
use crate::{field, read_form, require, Form};
#[cfg(feature = "api")]
use cat::handle_cat;
#[cfg(feature = "api")]
use cp::handle_cp;
#[cfg(feature = "api")]
use download::handle_download;
#[cfg(feature = "api")]
use find::handle_find;
#[cfg(feature = "api")]
use hyper::body::Incoming;
#[cfg(feature = "api")]
use hyper::http::request::Parts;
#[cfg(feature = "api")]
use hyper::http::Method;
#[cfg(feature = "api")]
use mkdir::handle_mkdir;
#[cfg(feature = "api")]
use mv::handle_mv;
#[cfg(feature = "api")]
use rm::handle_rm;
#[cfg(feature = "api")]
use upload::handle_upload;

#[cfg(feature = "api")]
const SMALL_FORM_LIMIT: usize = 64 * 1024;
#[cfg(feature = "api")]
const DOWNLOAD_FORM_LIMIT: usize = 1024 * 1024;

#[cfg(feature = "api")]
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

#[cfg(feature = "api")]
fn into_response(result: Result<Response, Response>) -> Response {
    match result {
        Ok(response) | Err(response) => response,
    }
}

#[cfg(feature = "api")]
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
