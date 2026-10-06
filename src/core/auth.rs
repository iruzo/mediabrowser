use crate::response::{self, Response};
use crate::types::{data_dir, path_metadata};
use crate::Access;
use argon2::password_hash::Output;
use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordVerifier, Version};
use hyper::http::{HeaderMap, StatusCode};
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

const COOKIE: &str = "mediabrowser_session";

pub(crate) struct Auth {
    pub(crate) sessions: Arc<Mutex<HashMap<String, String>>>,
    pub(crate) login: Arc<Semaphore>,
}

impl Auth {
    pub(crate) fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            login: Arc::new(Semaphore::new(1)),
        }
    }

    pub(crate) async fn session(&self, headers: &HeaderMap) -> Option<Access> {
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
}

pub(crate) fn set_cookie(response: &mut Response, token: &str, clear: bool) {
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

pub(crate) fn cookie(headers: &HeaderMap) -> Option<&str> {
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

pub(crate) fn same_origin(headers: &HeaderMap) -> bool {
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

pub(crate) async fn account(username: &str) -> Option<PathBuf> {
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

pub(crate) fn replace_password(directory: &Path, password: &[u8; 64]) -> io::Result<()> {
    let record = password_record(password)?;
    let temporary = directory.join(".shadow.new");
    save_password(&temporary, &record)?;
    let result = std::fs::rename(&temporary, directory.join(".shadow"));
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

pub(crate) fn verify(directory: &Path, password: &[u8; 64], setup: bool) -> io::Result<bool> {
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

pub(crate) fn login_required() -> Response {
    response::text(StatusCode::UNAUTHORIZED, "Login required")
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
