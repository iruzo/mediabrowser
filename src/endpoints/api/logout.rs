use crate::core::auth::{cookie, set_cookie, Auth};
use crate::response::{self, Response};
use hyper::body::Incoming;
use hyper::http::{Method, Request, StatusCode};

pub(crate) fn handle_logout(auth: &Auth, request: &Request<Incoming>) -> Response {
    if request.method() != Method::GET {
        return response::status(StatusCode::METHOD_NOT_ALLOWED);
    }
    if let Some(token) = cookie(request.headers()) {
        auth.sessions.lock().expect("session lock").remove(token);
    }
    let mut response = response::status(StatusCode::OK);
    set_cookie(&mut response, "", true);
    response
}
