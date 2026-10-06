use sha2::{Digest, Sha512};

// Decode as chunks arrive so password length does not determine memory use.
pub(super) struct AuthForm {
    name: Vec<u8>,
    value: bool,
    escape: bool,
    high: Option<u8>,
    username: Vec<u8>,
    password: Sha512,
    new_password: Sha512,
    changing: bool,
    has_new_password: bool,
    new_password_empty: bool,
    has_username: bool,
    has_password: bool,
    password_empty: bool,
}

impl AuthForm {
    pub(super) fn new(changing: bool) -> Self {
        Self {
            name: Vec::new(),
            value: false,
            escape: false,
            high: None,
            username: Vec::new(),
            password: Sha512::new(),
            new_password: Sha512::new(),
            changing,
            has_new_password: false,
            new_password_empty: true,
            has_username: false,
            has_password: false,
            password_empty: true,
        }
    }

    pub(super) fn push(&mut self, bytes: &[u8]) -> Result<(), ()> {
        for &byte in bytes {
            if byte == b'&' {
                self.end_field()?;
                self.name.clear();
                self.value = false;
            } else if byte == b'=' && !self.value {
                if self.name != b"password"
                    && self.name
                        != if self.changing {
                            b"new_password".as_slice()
                        } else {
                            b"username".as_slice()
                        }
                {
                    return Err(());
                }
                if (self.name == b"username" && self.has_username)
                    || (self.name == b"password" && self.has_password)
                    || (self.name == b"new_password" && self.has_new_password)
                {
                    return Err(());
                }
                self.value = true;
            } else if !self.value {
                if self.name.len() == 12 {
                    return Err(());
                }
                self.name.push(byte);
            } else {
                let decoded = if self.escape {
                    let digit = (byte as char).to_digit(16).ok_or(())? as u8;
                    if let Some(high) = self.high.take() {
                        self.escape = false;
                        high * 16 + digit
                    } else {
                        self.high = Some(digit);
                        continue;
                    }
                } else {
                    match byte {
                        b'%' => {
                            self.escape = true;
                            continue;
                        }
                        b'+' => b' ',
                        _ => byte,
                    }
                };
                if self.name == b"username" {
                    if self.username.len() == 255 {
                        return Err(());
                    }
                    self.username.push(decoded);
                } else if self.name == b"new_password" {
                    self.new_password.update([decoded]);
                    self.new_password_empty = false;
                } else {
                    self.password.update([decoded]);
                    self.password_empty = false;
                }
            }
        }
        Ok(())
    }

    fn end_field(&mut self) -> Result<(), ()> {
        if !self.value || self.escape {
            return Err(());
        }
        match self.name.as_slice() {
            b"username" => self.has_username = true,
            b"password" => self.has_password = true,
            b"new_password" => self.has_new_password = true,
            _ => return Err(()),
        }
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<Credentials, ()> {
        self.end_field()?;
        if !self.has_password
            || self.password_empty
            || (self.changing && (!self.has_new_password || self.new_password_empty))
            || (!self.changing && !self.has_username)
        {
            return Err(());
        }
        let username = String::from_utf8(self.username).map_err(|_| ())?;
        Ok(Credentials {
            username,
            password: self.password.finalize().into(),
            new_password: self.new_password.finalize().into(),
        })
    }
}

pub(super) struct Credentials {
    pub username: String,
    pub password: [u8; 64],
    pub new_password: [u8; 64],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_across_every_chunk_boundary() {
        let body = b"password=+a%26%3D%25%C3%B1+&username=alice";
        for split in 0..=body.len() {
            let mut form = AuthForm::new(false);
            form.push(&body[..split]).unwrap();
            form.push(&body[split..]).unwrap();
            let Credentials {
                username, password, ..
            } = form.finish().unwrap();
            assert_eq!(username, "alice");
            assert_eq!(password.as_slice(), Sha512::digest(" a&=%ñ ").as_slice());
        }
    }

    #[test]
    fn accepts_long_passwords_without_retaining_them() {
        let mut form = AuthForm::new(false);
        form.push(b"username=alice&password=").unwrap();
        let chunk = [b'x'; 8192];
        let mut expected = Sha512::new();
        for _ in 0..1024 {
            form.push(&chunk).unwrap();
            expected.update(chunk);
        }
        assert_eq!(
            form.finish().unwrap().password.as_slice(),
            expected.finalize().as_slice()
        );
    }

    #[test]
    fn decodes_password_changes_across_every_chunk_boundary() {
        let body = b"new_password=+n%26%3D%C3%B1+&password=old%25+";
        for split in 0..=body.len() {
            let mut form = AuthForm::new(true);
            form.push(&body[..split]).unwrap();
            form.push(&body[split..]).unwrap();
            let form = form.finish().unwrap();
            assert_eq!(form.password.as_slice(), Sha512::digest("old% ").as_slice());
            assert_eq!(
                form.new_password.as_slice(),
                Sha512::digest(" n&=ñ ").as_slice()
            );
        }
    }

    #[test]
    fn rejects_ambiguous_or_incomplete_forms() {
        for body in [
            "username=a&password=x&password=y",
            "username=a&username=b&password=x",
            "username=a&password=%",
            "username=a&password=%x0",
            "username=a&password=",
            "password=x",
            "username=a&other=x",
        ] {
            let mut form = AuthForm::new(false);
            assert!(
                form.push(body.as_bytes())
                    .and_then(|_| form.finish())
                    .is_err(),
                "{body}"
            );
        }
    }
}
