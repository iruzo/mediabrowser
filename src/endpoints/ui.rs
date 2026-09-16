use crate::resolve_path;
use crate::response::{self, Response};
use bytes::Bytes;
use hyper::http::header::{ACCEPT_ENCODING, CONTENT_ENCODING, LOCATION, VARY};
use hyper::http::{HeaderMap, HeaderValue, StatusCode, Uri};

const PAGE_GZIP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/ui.html.gz"));

#[cfg(test)]
const PAGE: &str = include_str!(concat!(env!("OUT_DIR"), "/ui.html"));

fn parse_quality(value: &str) -> Option<u16> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if fraction.len() > 3 || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if whole == "1" {
        return fraction.bytes().all(|byte| byte == b'0').then_some(1000);
    }
    if whole != "0" {
        return None;
    }

    let mut quality = 0;
    for byte in fraction.bytes() {
        quality = quality * 10 + u16::from(byte - b'0');
    }
    for _ in fraction.len()..3 {
        quality *= 10;
    }
    Some(quality)
}

fn parse_coding(value: &str) -> Option<(&str, u16)> {
    let mut parts = value.split(';');
    let coding = parts.next()?.trim();
    if coding.is_empty() {
        return None;
    }

    let mut quality = 1000;
    let mut has_quality = false;
    for parameter in parts {
        let Some((name, value)) = parameter.split_once('=') else {
            return Some((coding, 0));
        };
        if name.trim().eq_ignore_ascii_case("q") {
            if has_quality {
                return Some((coding, 0));
            }
            let Some(value) = parse_quality(value.trim()) else {
                return Some((coding, 0));
            };
            quality = value;
            has_quality = true;
        }
    }
    Some((coding, quality))
}

fn accepts_gzip(headers: &HeaderMap) -> bool {
    if !headers.contains_key(ACCEPT_ENCODING) {
        return true;
    }

    let mut gzip = None;
    let mut wildcard = None;
    for value in headers.get_all(ACCEPT_ENCODING) {
        let Ok(value) = value.to_str() else {
            continue;
        };
        for value in value.split(',') {
            let Some((coding, quality)) = parse_coding(value) else {
                continue;
            };
            if coding.eq_ignore_ascii_case("gzip") || coding.eq_ignore_ascii_case("x-gzip") {
                gzip = Some(gzip.unwrap_or(true) && quality != 0);
            } else if coding == "*" {
                wildcard = Some(wildcard.unwrap_or(true) && quality != 0);
            }
        }
    }
    gzip.or(wildcard).unwrap_or(false)
}

/// Returns the gzip UI representation without request-header negotiation.
pub fn handle_ui() -> Response {
    let mut response = response::html(Bytes::from_static(PAGE_GZIP));
    response
        .headers_mut()
        .insert(CONTENT_ENCODING, HeaderValue::from_static("gzip"));
    response
        .headers_mut()
        .insert(VARY, HeaderValue::from_static("Accept-Encoding"));
    response
}

/// Negotiates the gzip UI representation for an HTTP request.
pub fn handle_ui_request(headers: &HeaderMap) -> Response {
    if accepts_gzip(headers) {
        return handle_ui();
    }

    let mut response = response::status(StatusCode::NOT_ACCEPTABLE);
    response
        .headers_mut()
        .insert(VARY, HeaderValue::from_static("Accept-Encoding"));
    response
}

fn redirect(status: StatusCode, path: &str, query: Option<&str>) -> Response {
    let mut location = path.to_string();
    if let Some(query) = query {
        location.push('?');
        location.push_str(query);
    }

    let mut response = response::status(status);
    response.headers_mut().insert(
        LOCATION,
        HeaderValue::from_str(&location).expect("valid redirect URI"),
    );
    response
}

pub(crate) async fn handle_ui_path(uri: &Uri, headers: &HeaderMap) -> Response {
    let path = uri.path();
    if path == "/ui" {
        return redirect(StatusCode::PERMANENT_REDIRECT, "/ui/", uri.query());
    }
    if path == "/ui/" {
        return handle_ui_request(headers);
    }

    let requested_path = path.strip_prefix("/ui/").unwrap_or_default();
    let (_, metadata) = match resolve_path(requested_path).await {
        Ok(path) => path,
        Err(response) => return response,
    };

    handle_ui_entry(uri, headers, &metadata)
}

fn handle_ui_entry(uri: &Uri, headers: &HeaderMap, metadata: &std::fs::Metadata) -> Response {
    let path = uri.path();

    if metadata.is_dir() {
        if path.ends_with('/') {
            return handle_ui_request(headers);
        }

        let mut canonical = path.to_string();
        canonical.push('/');
        return redirect(StatusCode::TEMPORARY_REDIRECT, &canonical, uri.query());
    }

    if path.ends_with('/') {
        return response::text(StatusCode::NOT_FOUND, "Not found");
    }
    if metadata.is_file() {
        return handle_ui_request(headers);
    }

    response::text(StatusCode::NOT_FOUND, "Not found")
}

#[cfg(test)]
mod tests {
    use super::{accepts_gzip, handle_ui_entry, handle_ui_request, redirect, PAGE, PAGE_GZIP};
    use crate::response::Body;
    use flate2::read::GzDecoder;
    use hyper::http::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE, LOCATION, VARY};
    use hyper::http::{HeaderMap, HeaderValue, StatusCode};
    use std::io::Read;

    fn headers(values: &[&'static str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for value in values {
            headers.append(ACCEPT_ENCODING, HeaderValue::from_static(value));
        }
        headers
    }

    fn attribute_count(name: &str, value: &str) -> usize {
        let double_quoted = format!(r#"{name}="{value}""#);
        let single_quoted = format!("{name}='{value}'");
        let unquoted = format!("{name}={value}");
        PAGE.matches(&double_quoted).count()
            + PAGE.matches(&single_quoted).count()
            + PAGE.matches(&unquoted).count()
    }

    #[test]
    fn embeds_the_client_ui() {
        assert_eq!(attribute_count("id", "directories"), 1);
        assert_eq!(attribute_count("name", "directories"), 0);
        assert_eq!(attribute_count("id", "action-menu"), 1);
        assert_eq!(attribute_count("id", "selection-menu"), 1);
        assert_eq!(attribute_count("id", "viewer-root"), 1);
        assert_eq!(attribute_count("id", "viewer-template"), 0);
        assert_eq!(attribute_count("id", "viewer-menu"), 1);
        assert_eq!(attribute_count("id", "viewer-download"), 1);
        assert_eq!(attribute_count("class", "popover"), 3);
        assert_eq!(attribute_count("popover", "manual"), 2);
        assert_eq!(attribute_count("popover", "auto"), 1);
        assert_eq!(attribute_count("id", "prev"), 1);
        assert_eq!(attribute_count("id", "close"), 1);
        assert_eq!(attribute_count("id", "next"), 1);
        assert_eq!(attribute_count("id", "loopstart"), 1);
        assert_eq!(attribute_count("id", "loopend"), 1);
        assert_eq!(attribute_count("id", "loopclear"), 1);
        assert_eq!(attribute_count("id", "menu"), 1);
        assert_eq!(attribute_count("class", "download"), 1);
        assert_eq!(attribute_count("class", "to cp-to"), 1);
        assert_eq!(attribute_count("class", "actions"), 0);
        assert_eq!(attribute_count("data-zoom", "reset"), 1);
        assert!(
            PAGE.contains(r#"id="viewer-root" hidden"#)
                || PAGE.contains("id='viewer-root' hidden")
                || PAGE.contains("id=viewer-root hidden")
        );
        assert!(PAGE.contains("IntersectionObserver"));
        assert!(PAGE.contains("metadata"));
        assert!(PAGE.contains("/api/cat"));
        assert!(PAGE.contains("URL.createObjectURL"));
        assert!(!PAGE.contains("function fileUrl"));
        assert!(!PAGE.contains("{{CSS}}"));
        assert!(!PAGE.contains("UI_SCRIPT"));
    }

    #[test]
    fn generates_deterministic_gzip_metadata() {
        assert_eq!(&PAGE_GZIP[..4], &[0x1f, 0x8b, 0x08, 0x00]);
        assert_eq!(&PAGE_GZIP[4..8], &[0, 0, 0, 0]);
        assert_eq!(PAGE_GZIP[8], 2);
        assert_eq!(PAGE_GZIP[9], 255);
        assert!(PAGE_GZIP.len() < PAGE.len());

        let mut page = String::new();
        GzDecoder::new(PAGE_GZIP).read_to_string(&mut page).unwrap();
        assert_eq!(page, PAGE);

        let end = PAGE_GZIP.len();
        let size = u32::from_le_bytes(PAGE_GZIP[end - 4..end].try_into().unwrap());
        assert_eq!(size as usize, PAGE.len());
    }

    #[test]
    fn negotiates_gzip_encoding() {
        assert!(accepts_gzip(&HeaderMap::new()));
        assert!(!accepts_gzip(&headers(&[""])));
        assert!(accepts_gzip(&headers(&["gzip"])));
        assert!(accepts_gzip(&headers(&["br, GZip; q=0.001"])));
        assert!(accepts_gzip(&headers(&["x-gzip"])));
        assert!(!accepts_gzip(&headers(&["gzip;q=0"])));
        assert!(accepts_gzip(&headers(&["*;q=0.5"])));
        assert!(!accepts_gzip(&headers(&["gzip;q=0, *;q=1"])));
        assert!(accepts_gzip(&headers(&["br", "gzip"])));
        assert!(!accepts_gzip(&headers(&["gzip;q=invalid"])));
        assert!(!accepts_gzip(&headers(&["gzip;q=invalid, *;q=1"])));
        assert!(!accepts_gzip(&headers(&["gzip;q=1.001"])));
        assert!(!accepts_gzip(&headers(&["gzip;q=0, gzip;q=1"])));
        assert!(!accepts_gzip(&headers(&["gzip;q=1, gzip;q=0"])));
        assert!(!accepts_gzip(&headers(&["gzip;q=0;q=1"])));
        assert!(!accepts_gzip(&headers(&["gzip;q=1;q=0"])));
        assert!(!accepts_gzip(&headers(&["x-gzip;q=0, *;q=1"])));
    }

    #[test]
    fn serves_only_the_gzip_representation() {
        let response = handle_ui_request(&headers(&["gzip"]));
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "text/html; charset=utf-8");
        assert_eq!(response.headers()[CONTENT_ENCODING], "gzip");
        assert_eq!(response.headers()[VARY], "Accept-Encoding");
        let Body::Full(Some(body)) = response.body() else {
            panic!("expected a full response body");
        };
        assert_eq!(body.as_ref(), PAGE_GZIP);

        let response = handle_ui_request(&headers(&["gzip;q=0"]));
        assert_eq!(response.status(), StatusCode::NOT_ACCEPTABLE);
        assert_eq!(response.headers()[VARY], "Accept-Encoding");
        assert!(!response.headers().contains_key(CONTENT_ENCODING));
        assert!(matches!(response.body(), Body::Empty));
    }

    #[test]
    fn builds_same_origin_redirects_with_queries() {
        let response = redirect(
            StatusCode::TEMPORARY_REDIRECT,
            "/folder/file.txt",
            Some("download=true"),
        );
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            response.headers()[LOCATION],
            "/folder/file.txt?download=true"
        );
        assert!(matches!(response.body(), Body::Empty));
    }

    #[test]
    fn routes_resolved_ui_paths() {
        let executable = std::env::current_exe().unwrap();
        let file = std::fs::metadata(executable).unwrap();
        let directory = std::fs::metadata(std::env::current_dir().unwrap()).unwrap();
        let headers = headers(&["gzip"]);

        let uri = "/ui/photos?sort=name".parse().unwrap();
        let response = handle_ui_entry(&uri, &headers, &directory);
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(response.headers()[LOCATION], "/ui/photos/?sort=name");
        assert!(!response.headers().contains_key(CONTENT_ENCODING));
        assert!(!response.headers().contains_key(VARY));

        let uri = "/ui/photos.jpg/".parse().unwrap();
        let response = handle_ui_entry(&uri, &headers, &directory);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_ENCODING], "gzip");

        let uri = "/ui/photos/image%20%23%3F.jpg".parse().unwrap();
        let response = handle_ui_entry(&uri, &headers, &file);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_ENCODING], "gzip");

        let uri = "/ui//notes.txt?download=true".parse().unwrap();
        let response = handle_ui_entry(&uri, &headers, &file);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_ENCODING], "gzip");
        assert_eq!(response.headers()[VARY], "Accept-Encoding");

        let uri = "/ui/notes.txt/".parse().unwrap();
        let response = handle_ui_entry(&uri, &headers, &file);
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
