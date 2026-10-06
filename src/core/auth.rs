mod form;

use super::access::Access;
use super::types::{data_dir, path_metadata};
use crate::response::{self, Response};
use argon2::password_hash::Output;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordVerifier, Version};
use form::{AuthForm, Credentials};
use http_body::Body;
use hyper::body::Incoming;
use hyper::http::{HeaderMap, Method, Request, StatusCode};
use std::collections::HashMap;
use std::future::poll_fn;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

const COOKIE: &str = "mediabrowser_session";
const PAGE: &str = include_str!("../../static/login.html");
const PASSWORD_PAGE: &str = include_str!("../../static/password.html");

pub(super) struct Auth {
    sessions: Arc<Mutex<HashMap<String, String>>>,
    login: Arc<Semaphore>,
}

impl Auth {
    pub(super) fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            login: Arc::new(Semaphore::new(1)),
        }
    }

    pub(super) async fn session(&self, headers: &HeaderMap) -> Option<Access> {
        let token = cookie(headers)?;
        let username = self.sessions.lock().ok()?.get(token)?.clone();
        let directory = account(&username).await?;
        if !path_metadata(&directory.join(".shadow"))
            .await
            .ok()?
            .is_file()
        {
            return None;
        }
        Some(Access { username })
    }

    pub(super) async fn login(&self, request: Request<Incoming>) -> Response {
        if request.method() == Method::GET {
            return response::html(PAGE);
        }
        if request.method() != Method::POST {
            return response::status(StatusCode::METHOD_NOT_ALLOWED);
        }
        if !same_origin(request.headers()) {
            return response::text(StatusCode::FORBIDDEN, "Access denied");
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
        let Ok(permit) = self.login.clone().acquire_owned().await else {
            return denied();
        };
        let valid =
            tokio::task::spawn_blocking(move || (verify(&directory, &password, true), permit))
                .await;
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
        let mut sessions = self.sessions.lock().expect("session lock");
        if let Some(previous) = previous {
            sessions.remove(&previous);
        }
        sessions.insert(token.clone(), username);
        let destination = if cfg!(feature = "ui") { "/ui/" } else { "/" };
        let mut response = redirect(destination);
        set_cookie(&mut response, &token, false);
        response
    }
    pub(super) async fn password(&self, request: Request<Incoming>, access: &Access) -> Response {
        if request.method() == Method::GET {
            return response::html(PASSWORD_PAGE);
        }
        if request.method() != Method::POST {
            return response::status(StatusCode::METHOD_NOT_ALLOWED);
        }
        let token = cookie(request.headers()).unwrap_or("").to_owned();
        let form = match read_credentials(request, true).await {
            Ok(form) => form,
            Err(response) => return response,
        };
        let Some(directory) = account(&access.username).await else {
            return redirect("/api/login");
        };
        let Ok(permit) = self.login.clone().acquire_owned().await else {
            return response::status(StatusCode::INTERNAL_SERVER_ERROR);
        };
        let sessions = self.sessions.clone();
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
            Ok(Ok(true)) => redirect(if cfg!(feature = "ui") { "/ui/" } else { "/" }),
            Ok(Ok(false)) => response::text(StatusCode::UNAUTHORIZED, "Password change denied"),
            _ => response::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Password change unavailable",
            ),
        }
    }

    pub(super) fn logout(&self, request: &Request<Incoming>) -> Response {
        if request.method() != Method::GET {
            return response::status(StatusCode::METHOD_NOT_ALLOWED);
        }
        if let Some(token) = cookie(request.headers()) {
            self.sessions.lock().expect("session lock").remove(token);
        }
        let mut response = redirect("/api/login");
        set_cookie(&mut response, "", true);
        response
    }
}

async fn read_credentials(
    request: Request<Incoming>,
    changing: bool,
) -> Result<Credentials, Response> {
    let content_type = request
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case("application/x-www-form-urlencoded")
    {
        return Err(response::status(StatusCode::UNSUPPORTED_MEDIA_TYPE));
    }
    let mut body = request.into_body();
    let mut form = AuthForm::new(changing);
    while let Some(frame) = poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await {
        let Ok(frame) = frame else {
            return Err(response::status(StatusCode::BAD_REQUEST));
        };
        if let Ok(bytes) = frame.into_data() {
            if form.push(&bytes).is_err() {
                return Err(response::status(StatusCode::BAD_REQUEST));
            }
        }
    }
    form.finish()
        .map_err(|_| response::status(StatusCode::BAD_REQUEST))
}

fn set_cookie(response: &mut Response, token: &str, clear: bool) {
    let secure = if cfg!(feature = "https") {
        "; Secure"
    } else {
        ""
    };
    let expiration = if clear { "; Max-Age=0" } else { "" };
    response.headers_mut().insert(
        "set-cookie",
        format!("{COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict{secure}{expiration}")
            .parse()
            .expect("valid cookie"),
    );
}

fn cookie(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all("cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|v| {
            let (name, value) = v.trim().split_once('=')?;
            (name == COOKIE && value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
                .then_some(value)
        })
}

pub(super) fn same_origin(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get("origin") else {
        return true;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Some(host) = headers.get("host").and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let scheme = if cfg!(feature = "https") {
        "https"
    } else {
        "http"
    };
    origin.eq_ignore_ascii_case(&format!("{scheme}://{host}"))
}

fn account_path(username: &str) -> Option<PathBuf> {
    if username.is_empty()
        || matches!(username, "." | ".." | ".shadow")
        || username
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return None;
    }
    Some(if username == "root" {
        data_dir().join(".root")
    } else {
        data_dir().join(".home").join(username)
    })
}

async fn account(username: &str) -> Option<PathBuf> {
    let directory = account_path(username)?;
    path_metadata(&directory)
        .await
        .ok()?
        .is_dir()
        .then_some(directory)
}

fn argon2() -> Argon2<'static> {
    let params = Params::new(19456, 2, 1, Some(64)).expect("valid Argon2 parameters");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

fn password_record(password: &[u8; 64]) -> io::Result<[u8; 80]> {
    let mut record = [0; 80];
    let (salt, hash) = record.split_at_mut(16);
    getrandom::fill(salt).map_err(|e| io::Error::other(e.to_string()))?;
    argon2()
        .hash_password_into(password, salt, hash)
        .map_err(|e| io::Error::other(e.to_string()))?;
    Ok(record)
}

fn save_password(path: &Path, record: &[u8; 80]) -> io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(record)?;
    file.sync_all()
}

fn replace_password(directory: &Path, password: &[u8; 64]) -> io::Result<()> {
    let record = password_record(password)?;
    let temporary = directory.join(".shadow.new");
    save_password(&temporary, &record)?;
    let result = std::fs::rename(&temporary, directory.join(".shadow"));
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn verify(directory: &Path, password: &[u8; 64], setup: bool) -> io::Result<bool> {
    let path = directory.join(".shadow");
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => {
            let length = metadata.len();
            if length > 1024 {
                return Ok(false);
            }
            let mut stored = [0; 1024];
            let stored = &mut stored[..length as usize];
            std::fs::File::open(&path)?.read_exact(stored)?;
            if stored.len() == 80 {
                let mut hash = [0; 64];
                argon2()
                    .hash_password_into(password, &stored[..16], &mut hash)
                    .map_err(|e| io::Error::other(e.to_string()))?;
                // Output compares hashes in constant time.
                return Ok(Output::new(&hash).expect("64-byte hash")
                    == Output::new(&stored[16..]).expect("64-byte hash"));
            }
            // Convert the previous text format only after verifying the password.
            let Ok(value) = std::str::from_utf8(stored) else {
                return Ok(false);
            };
            let Some(value) = value.strip_prefix("sha512:") else {
                return Ok(false);
            };
            let Ok(hash) = PasswordHash::new(value.trim_end()) else {
                return Ok(false);
            };
            if Argon2::default().verify_password(password, &hash).is_err() {
                return Ok(false);
            }
            replace_password(directory, password).map(|_| true)
        }
        Ok(_) => Ok(false),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if !setup {
                return Ok(false);
            }
            match save_password(&path, &password_record(password)?) {
                Ok(()) => Ok(true),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(false),
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

fn denied() -> Response {
    let mut response = response::html(PAGE);
    *response.status_mut() = StatusCode::UNAUTHORIZED;
    response
}

pub(super) fn redirect(path: &'static str) -> Response {
    hyper::http::Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header("location", path)
        .body(response::empty())
        .expect("valid redirect")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha512};

    #[test]
    fn creates_salted_512_bit_hash_and_preserves_first_password() {
        let mut random = [0; 8];
        getrandom::fill(&mut random).unwrap();
        let directory =
            std::env::temp_dir().join(format!("mediabrowser-auth-{}", u64::from_ne_bytes(random)));
        std::fs::create_dir(&directory).unwrap();
        let password = Sha512::digest(b"first password").into();
        assert!(!verify(&directory, &password, false).unwrap());
        assert!(!directory.join(".shadow").exists());
        assert!(verify(&directory, &password, true).unwrap());
        let stored = std::fs::read(directory.join(".shadow")).unwrap();
        assert_eq!(stored.len(), 80);
        let second = password_record(&password).unwrap();
        assert_ne!(&stored[..16], &second[..16]);
        assert_ne!(&stored[16..], &second[16..]);
        assert!(verify(&directory, &password, true).unwrap());
        assert!(!verify(&directory, &Sha512::digest(b"different").into(), true).unwrap());
        assert_eq!(std::fs::read(directory.join(".shadow")).unwrap(), stored);
        use argon2::{password_hash::SaltString, PasswordHasher};
        let salt = SaltString::encode_b64(&[1; 16]).unwrap();
        let hash = argon2()
            .hash_password(&password, &salt)
            .unwrap()
            .to_string();
        let old = format!("sha512:{hash}\n");
        std::fs::write(directory.join(".shadow"), &old).unwrap();
        assert!(!verify(&directory, &Sha512::digest(b"wrong").into(), true).unwrap());
        assert_eq!(
            std::fs::read_to_string(directory.join(".shadow")).unwrap(),
            old
        );
        assert!(verify(&directory, &password, true).unwrap());
        assert_eq!(
            std::fs::metadata(directory.join(".shadow")).unwrap().len(),
            80
        );
        assert!(!directory.join(".shadow.new").exists());
        assert!(verify(&directory, &password, true).unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(directory.join(".shadow"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            std::fs::remove_file(directory.join(".shadow")).unwrap();
            std::os::unix::fs::symlink("missing", directory.join(".shadow")).unwrap();
            assert!(!verify(&directory, &password, true).unwrap());
            assert!(!directory.join("missing").exists());
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}
