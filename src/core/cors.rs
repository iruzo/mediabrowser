use crate::response::{self, Response};
use hyper::http::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_REQUEST_METHOD,
    ORIGIN,
};
use hyper::http::{HeaderValue, Method, Request, StatusCode, Uri};
use std::error::Error;

pub(super) fn origin() -> Result<Option<HeaderValue>, Box<dyn Error>> {
    match std::env::var("CORS_ORIGIN") {
        Ok(value) => parse_origin(&value),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn parse_origin(value: &str) -> Result<Option<HeaderValue>, Box<dyn Error>> {
    if value.is_empty() {
        return Ok(None);
    }
    if value != "*" {
        let uri: Uri = value.parse()?;
        let scheme = uri.scheme_str().unwrap_or_default();
        let authority = uri
            .authority()
            .map(|value| value.as_str())
            .unwrap_or_default();
        if !matches!(scheme, "http" | "https")
            || authority.is_empty()
            || authority.contains('@')
            || value.len() != scheme.len() + 3 + authority.len()
        {
            return Err("CORS_ORIGIN must be * or one HTTP(S) origin without a path".into());
        }
    }
    Ok(Some(HeaderValue::from_str(value)?))
}

pub(super) fn preflight<B>(request: &Request<B>) -> Option<Response> {
    if request.method() != Method::OPTIONS
        || !request.headers().contains_key(ORIGIN)
        || !request
            .headers()
            .contains_key(ACCESS_CONTROL_REQUEST_METHOD)
    {
        return None;
    }

    let mut response = response::status(StatusCode::NO_CONTENT);
    let headers = response.headers_mut();
    headers.insert(
        ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST"),
    );
    headers.insert(
        ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type, Range"),
    );
    Some(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_empty_wildcard_and_single_origins() {
        assert!(parse_origin("").unwrap().is_none());
        for value in [
            "*",
            "https://example.com",
            "http://localhost:8080",
            "http://[::1]:8080",
        ] {
            assert_eq!(parse_origin(value).unwrap().unwrap(), value);
        }
    }

    #[test]
    fn rejects_invalid_origins() {
        for value in [
            "null",
            "example.com",
            "ftp://example.com",
            "https://example.com/",
            "https://example.com/path",
            "https://example.com?query",
            "https://example.com#fragment",
            "https://user@example.com",
            "https://one.example,https://two.example",
            "https://example.com\r\nx-injected: value",
        ] {
            assert!(parse_origin(value).is_err(), "accepted {value:?}");
        }
    }

    #[test]
    fn handles_only_cors_preflights() {
        for (method, origin, requested_method, expected) in [
            ("OPTIONS", true, true, true),
            ("OPTIONS", false, true, false),
            ("OPTIONS", true, false, false),
            ("GET", true, true, false),
            ("POST", true, false, false),
        ] {
            let mut request = Request::builder().method(method);
            if origin {
                request = request.header(ORIGIN, "https://example.com");
            }
            if requested_method {
                request = request.header(ACCESS_CONTROL_REQUEST_METHOD, "POST");
            }
            let response = preflight(&request.body(()).unwrap());
            assert_eq!(response.is_some(), expected);
            if let Some(response) = response {
                assert_eq!(response.status(), StatusCode::NO_CONTENT);
                assert_eq!(
                    response.headers()[ACCESS_CONTROL_ALLOW_METHODS],
                    "GET, POST"
                );
                assert_eq!(
                    response.headers()[ACCESS_CONTROL_ALLOW_HEADERS],
                    "Content-Type, Range"
                );
            }
        }
    }
}
