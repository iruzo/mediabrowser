use crate::core::auth::{account, cookie, login_required, replace_password, verify, Auth};
use crate::core::credentials::read_credentials;
use crate::response::{self, Response};
use crate::Access;
use hyper::body::Incoming;
use hyper::http::{Method, Request, StatusCode};
use std::io;

pub(crate) async fn handle_password(
    auth: &Auth,
    request: Request<Incoming>,
    access: &Access,
) -> Response {
    if request.method() != Method::POST {
        return response::status(StatusCode::METHOD_NOT_ALLOWED);
    }
    let token = cookie(request.headers()).unwrap_or("").to_owned();
    let form = match read_credentials(request, true).await {
        Ok(form) => form,
        Err(response) => return response,
    };
    let Some(directory) = account(&access.username).await else {
        return login_required();
    };
    let Ok(permit) = auth.login.clone().acquire_owned().await else {
        return response::status(StatusCode::INTERNAL_SERVER_ERROR);
    };
    let sessions = auth.sessions.clone();
    let username = access.username.clone();
    // Complete replacement and session invalidation even if the client disconnects.
    let changed = tokio::task::spawn_blocking(move || -> io::Result<bool> {
        let _permit = permit;
        if sessions.lock().expect("session lock").get(&token) != Some(&username) {
            return Ok(false);
        }
        if !verify(&directory, &form.password, false)? {
            return Ok(false);
        }
        replace_password(&directory, &form.new_password)?;
        sessions
            .lock()
            .expect("session lock")
            .retain(|key, user| user != &username || key == &token);
        Ok(true)
    })
    .await;
    match changed {
        Ok(Ok(true)) => response::status(StatusCode::OK),
        Ok(Ok(false)) => response::text(StatusCode::UNAUTHORIZED, "Password change denied"),
        _ => response::text(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Password change unavailable",
        ),
    }
}
