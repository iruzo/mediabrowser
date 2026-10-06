use crate::response::{self, Response};
use hyper::http::StatusCode;
#[cfg(feature = "auth")]
use std::path::Component;
use std::path::Path;

#[derive(Clone, Default)]
pub struct Access {
    #[cfg(feature = "auth")]
    pub(crate) username: String,
}

impl Access {
    #[cfg(all(test, any(feature = "api", feature = "httpd")))]
    pub(crate) fn test_root() -> Self {
        Self {
            #[cfg(feature = "auth")]
            username: "root".into(),
        }
    }

    pub fn allows(&self, path: &Path) -> bool {
        #[cfg(feature = "auth")]
        {
            if self.username == "root" {
                return true;
            }
            if path.components().any(|part| {
                !matches!(part, Component::Normal(name) if name != ".shadow" && !name.to_string_lossy().starts_with(".shadow."))
                    && part != Component::CurDir
            }) {
                return false;
            }
            let mut parts = path.components().filter_map(|part| match part {
                Component::Normal(name) => Some(name),
                _ => None,
            });
            if self.username.is_empty() {
                return false;
            }
            match parts.next() {
                Some(name) if name == ".root" => false,
                Some(name) if name == ".home" => parts
                    .next()
                    .map_or(true, |name| name == self.username.as_str()),
                _ => true,
            }
        }
        #[cfg(not(feature = "auth"))]
        {
            let _ = path;
            true
        }
    }

    pub fn entry(&self, root: &Path, path: &Path) -> bool {
        path.strip_prefix(root).is_ok_and(|path| self.allows(path))
    }

    pub fn check(&self, path: &str, write: bool) -> Result<(), Response> {
        let path = Path::new(path.trim().trim_start_matches('/'));
        if !self.allows(path) || (write && self.protected(path)) {
            return Err(response::text(StatusCode::FORBIDDEN, "Access denied"));
        }
        Ok(())
    }

    fn protected(&self, path: &Path) -> bool {
        #[cfg(feature = "auth")]
        {
            if self.username == "root" {
                return false;
            }
            let mut parts = path.components().filter(|part| *part != Component::CurDir);
            match parts.next() {
                None => true,
                Some(Component::Normal(name)) if name == ".home" => {
                    parts.next();
                    parts.next().is_none()
                }
                Some(Component::Normal(name)) if name == ".root" => parts.next().is_none(),
                _ => false,
            }
        }
        #[cfg(not(feature = "auth"))]
        {
            let _ = path;
            false
        }
    }
}

#[cfg(all(test, feature = "auth"))]
mod tests {
    use super::Access;
    use std::path::Path;

    #[test]
    fn confines_users_and_protects_accounts() {
        let user = Access {
            username: "alice".into(),
        };
        for path in [
            "",
            "shared/file",
            ".home",
            ".home/alice",
            ".home/./alice/file",
        ] {
            assert!(user.allows(Path::new(path)), "{path}");
        }
        for path in [
            ".root",
            ".root/file",
            ".home/bob",
            ".home/alice/.shadow",
            "shared/.shadow",
            ".home/alice/.shadow.new",
            ".home/alice/../bob",
        ] {
            assert!(!user.allows(Path::new(path)), "{path}");
        }
        let root = Access {
            username: "root".into(),
        };
        assert!(root.allows(Path::new(".home/bob/file")));
        assert!(root.allows(Path::new(".root/file")));
        for path in [
            "/",
            ".",
            ".home",
            ".home/alice",
            ".root",
            ".root/.shadow",
            ".home/alice/.shadow",
            ".home/alice/.shadow.new",
        ] {
            assert!(root.check(path, false).is_ok(), "{path}");
            assert!(root.check(path, true).is_ok(), "{path}");
            assert!(user.check(path, true).is_err(), "{path}");
        }
        assert!(user.check(".home/alice/file", true).is_ok());
        assert!(!Access::default().allows(Path::new("shared")));
    }
}
