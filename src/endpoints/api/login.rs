use crate::core::auth::{account, cookie, same_origin, set_cookie, verify, Auth};
use crate::core::credentials::{read_credentials, Credentials};
use crate::response::{self, Response};
use hyper::body::Incoming;
use hyper::http::{Method, Request, StatusCode};

pub(crate) async fn handle_login(auth: &Auth, request: Request<Incoming>) -> Response {
    if request.method() != Method::POST {
        return response::status(StatusCode::METHOD_NOT_ALLOWED);
    }
    if !same_origin(request.headers()) {
        return response::text(StatusCode::NOT_FOUND, "Not found");
    }
    let previous = cookie(request.headers()).map(str::to_owned);
    let Credentials {
        username, password, ..
    } = match read_credentials(request, false).await {
        Ok(form) => form,
        Err(response) => return response,
    };
    let Some(directory) = account(&username).await else {
        return denied();
    };
    // Serialize password setup and hashing to bound memory and avoid competing first logins.
    let Ok(permit) = auth.login.clone().acquire_owned().await else {
        return denied();
    };
    let valid =
        tokio::task::spawn_blocking(move || (verify(&directory, &password, true), permit)).await;
    let _permit = match valid {
        Ok((Ok(true), permit)) => permit,
        Ok((Ok(false), _)) => return denied(),
        _ => return response::text(StatusCode::INTERNAL_SERVER_ERROR, "Login unavailable"),
    };
    let mut random = [0; 32];
    if getrandom::fill(&mut random).is_err() {
        return response::text(StatusCode::INTERNAL_SERVER_ERROR, "Login unavailable");
    }
    let mut token = String::with_capacity(64);
    for byte in random {
        use std::fmt::Write;
        write!(token, "{byte:02x}").expect("write token");
    }
    let mut sessions = auth.sessions.lock().expect("session lock");
    if let Some(previous) = previous {
        sessions.remove(&previous);
    }
    sessions.insert(token.clone(), username);
    let mut response = response::status(StatusCode::OK);
    set_cookie(&mut response, &token, false);
    response
}

fn denied() -> Response {
    response::text(StatusCode::UNAUTHORIZED, "Login denied")
}
