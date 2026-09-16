/// Streaming parser for multipart/form-data request bodies (RFC 7578).
use bytes::{Buf, Bytes, BytesMut};
use http_body::Body as HttpBody;
use hyper::body::Incoming;
use hyper::http::StatusCode;
use std::future::poll_fn;
use std::pin::Pin;

const MAX_HEADER_SIZE: usize = 8 * 1024;

type MultipartResult<T> = Result<T, (StatusCode, String)>;

#[derive(Default)]
pub struct PartHeader {
    pub name: Option<String>,
    pub filename: Option<String>,
}

pub struct Multipart {
    body: Incoming,
    buffer: BytesMut,
    delimiter: Vec<u8>,
    budget: u64,
    body_done: bool,
    finished: bool,
    in_part: bool,
}

pub fn parse_boundary(content_type: &str) -> Option<String> {
    let mut sections = content_type.split(';');

    if !sections
        .next()?
        .trim()
        .eq_ignore_ascii_case("multipart/form-data")
    {
        return None;
    }

    for section in sections {
        let Some(value) = section.trim().strip_prefix("boundary=") else {
            continue;
        };
        let value = value.strip_prefix('"').unwrap_or(value);
        let value = value.strip_suffix('"').unwrap_or(value);

        let valid = matches!(value.len(), 1..=70)
            && !value.ends_with(' ')
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric()
                    || matches!(
                        byte,
                        b'\''
                            | b'('
                            | b')'
                            | b'+'
                            | b'_'
                            | b','
                            | b'-'
                            | b'.'
                            | b'/'
                            | b':'
                            | b'='
                            | b'?'
                            | b' '
                    )
            });

        return valid.then(|| value.to_string());
    }

    None
}

impl Multipart {
    pub fn new(body: Incoming, boundary: &str, limit: u64) -> Self {
        Multipart {
            body,
            buffer: BytesMut::from(&b"\r\n"[..]),
            delimiter: format!("\r\n--{boundary}").into_bytes(),
            budget: limit,
            body_done: false,
            finished: false,
            in_part: true,
        }
    }

    pub async fn next_part(&mut self) -> MultipartResult<Option<PartHeader>> {
        while self.in_part {
            self.chunk().await?;
        }
        if self.finished {
            return Ok(None);
        }

        // chunk() consumed the delimiter and boundary line, so the buffer
        // now starts at this part's header block
        let header = self.read_headers().await?;
        self.in_part = true;
        Ok(Some(header))
    }

    pub async fn chunk(&mut self) -> MultipartResult<Option<Bytes>> {
        if !self.in_part {
            return Ok(None);
        }

        loop {
            if let Some(position) = find(&self.buffer, &self.delimiter) {
                let data = self.buffer.split_to(position).freeze();
                self.buffer.advance(self.delimiter.len());
                self.in_part = false;
                self.read_delimiter_suffix().await?;
                return Ok(if data.is_empty() { None } else { Some(data) });
            }

            let safe = self.buffer.len().saturating_sub(self.delimiter.len() - 1);
            if safe > 0 {
                let data = self.buffer.split_to(safe).freeze();
                return Ok(Some(data));
            }
            if !self.fill().await? {
                // body ended in the middle of a part: truncated request
                return Err(malformed());
            }
        }
    }

    async fn read_delimiter_suffix(&mut self) -> MultipartResult<()> {
        while self.buffer.len() < 2 {
            if !self.fill().await? {
                return Err(malformed());
            }
        }

        if self.buffer.starts_with(b"--") {
            self.finished = true;
            self.buffer.clear();
            return Ok(());
        }

        loop {
            if let Some(position) = find(&self.buffer, b"\r\n") {
                self.buffer.advance(position + 2);
                return Ok(());
            }
            if self.buffer.len() > MAX_HEADER_SIZE || !self.fill().await? {
                return Err(malformed());
            }
        }
    }

    async fn read_headers(&mut self) -> MultipartResult<PartHeader> {
        loop {
            if self.buffer.starts_with(b"\r\n") {
                self.buffer.advance(2);
                return Ok(PartHeader::default());
            }
            if let Some(position) = find(&self.buffer, b"\r\n\r\n") {
                let header = parse_part_header(&self.buffer[..position]);
                self.buffer.advance(position + 4);
                return header;
            }
            if self.buffer.len() > MAX_HEADER_SIZE || !self.fill().await? {
                return Err(malformed());
            }
        }
    }

    async fn fill(&mut self) -> MultipartResult<bool> {
        if self.body_done {
            return Ok(false);
        }

        match poll_fn(|cx| Pin::new(&mut self.body).poll_frame(cx)).await {
            None => {
                self.body_done = true;
                Ok(false)
            }
            Some(Err(error)) => Err((
                StatusCode::BAD_REQUEST,
                format!("Failed to process upload: {error}"),
            )),
            Some(Ok(frame)) => {
                if let Ok(data) = frame.into_data() {
                    self.budget = self.budget.checked_sub(data.len() as u64).ok_or((
                        StatusCode::BAD_REQUEST,
                        "Failed to process upload: upload is too large".to_string(),
                    ))?;
                    self.buffer.extend_from_slice(&data);
                }
                Ok(true)
            }
        }
    }
}

fn parse_part_header(header: &[u8]) -> MultipartResult<PartHeader> {
    let header = std::str::from_utf8(header).map_err(|_| malformed())?;

    for line in header.split("\r\n") {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("content-disposition") {
            continue;
        }

        return Ok(PartHeader {
            name: disposition_param(value, "name"),
            filename: disposition_param(value, "filename"),
        });
    }

    Ok(PartHeader::default())
}

fn disposition_param(value: &str, name: &str) -> Option<String> {
    let (_, mut rest) = value.split_once(';')?;

    loop {
        let (key, value) = rest.trim_start().split_once('=')?;
        let value = value.trim_start();
        let mut decoded = String::new();

        if let Some(value) = value.strip_prefix('"') {
            let mut chars = value.char_indices();
            loop {
                let (index, character) = chars.next()?;
                match character {
                    '"' => {
                        rest = value[index + 1..].trim_start();
                        break;
                    }
                    '\\' => decoded.push(chars.next()?.1),
                    _ => decoded.push(character),
                }
            }
        } else {
            let end = value.find(';').unwrap_or(value.len());
            decoded.push_str(value[..end].trim_end());
            rest = &value[end..];
        }

        if !rest.is_empty() && !rest.starts_with(';') {
            return None;
        }
        if key.trim().eq_ignore_ascii_case(name) {
            return Some(decoded);
        }
        rest = rest.strip_prefix(';')?;
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let first = *needle.first()?;
    let mut start = 0;

    while start + needle.len() <= haystack.len() {
        let offset = haystack[start..].iter().position(|&byte| byte == first)?;
        let position = start + offset;

        if position + needle.len() > haystack.len() {
            return None;
        }
        if &haystack[position..position + needle.len()] == needle {
            return Some(position);
        }
        start = position + 1;
    }

    None
}

fn malformed() -> (StatusCode, String) {
    (
        StatusCode::BAD_REQUEST,
        "Failed to process upload: malformed multipart body".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::{disposition_param, find, parse_boundary, parse_part_header};

    #[test]
    fn parses_boundaries() {
        assert_eq!(
            parse_boundary("multipart/form-data; boundary=xyz"),
            Some("xyz".to_string())
        );
        assert_eq!(
            parse_boundary("multipart/form-data; charset=utf-8; boundary=\"a b-c\""),
            Some("a b-c".to_string())
        );

        for content_type in [
            "",
            "text/plain",
            "multipart/form-data",
            "multipart/form-data; boundary=",
            "multipart/form-data; boundary=\"ends in space \"",
            "multipart/form-data; boundary=bad\"char",
        ] {
            assert_eq!(parse_boundary(content_type), None);
        }
    }

    #[test]
    fn parses_part_headers() {
        let header = parse_part_header(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"a b.txt\"\r\n\
              Content-Type: text/plain",
        )
        .unwrap();

        assert_eq!(header.name.as_deref(), Some("file"));
        assert_eq!(header.filename.as_deref(), Some("a b.txt"));

        let header = parse_part_header(b"Content-Type: text/plain").unwrap();
        assert_eq!(header.name, None);
        assert_eq!(header.filename, None);
    }

    #[test]
    fn matches_exact_disposition_params() {
        let value = "form-data; filename=\"x.txt\"; name=path";

        assert_eq!(disposition_param(value, "name"), Some("path".to_string()));
        assert_eq!(
            disposition_param(value, "filename"),
            Some("x.txt".to_string())
        );
        assert_eq!(disposition_param("form-data; filename=\"x\"", "name"), None);
    }

    #[test]
    fn preserves_quoted_filename_parameters() {
        for (value, expected) in [
            (
                r#"form-data; filename="report;final.txt"; name="file""#,
                "report;final.txt",
            ),
            (
                r#"form-data; filename="report;name=other.txt"; name="file""#,
                "report;name=other.txt",
            ),
            (
                r#"form-data; filename="report\";final.txt"; name="file""#,
                "report\";final.txt",
            ),
            (
                r#"form-data; filename="árbol;東京.txt"; name="file""#,
                "árbol;東京.txt",
            ),
        ] {
            assert_eq!(
                disposition_param(value, "filename").as_deref(),
                Some(expected)
            );
            assert_eq!(disposition_param(value, "name").as_deref(), Some("file"));
        }

        for value in [
            "form-data; filename=\"unfinished",
            "form-data; filename=\"unfinished\\",
            "form-data; filename=\"file.txt\"extra",
        ] {
            assert_eq!(disposition_param(value, "filename"), None);
        }
    }

    #[test]
    fn finds_needles_across_repeated_prefixes() {
        assert_eq!(find(b"aa\r\n--b", b"\r\n--b"), Some(2));
        assert_eq!(find(b"\r\r\n--b", b"\r\n--b"), Some(1));
        assert_eq!(find(b"\r\n--", b"\r\n--b"), None);
        assert_eq!(find(b"", b"\r\n"), None);
    }
}
