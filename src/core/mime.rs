#[cfg(feature = "httpd")]
use percent_encoding::{AsciiSet, CONTROLS};
use std::path::Path;

// Encode only characters that are not allowed in URL paths (matching Apache)
#[cfg(feature = "httpd")]
pub(crate) const PATH_SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'#')
    .add(b'?')
    .add(b'{')
    .add(b'}');

fn extension(path: &Path) -> Option<&str> {
    path.file_name()?
        .to_str()?
        .rsplit_once('.')
        .map(|(_, extension)| extension)
}

pub(crate) fn content_type(path: &Path) -> &'static str {
    let Some(extension) = extension(path) else {
        return "application/octet-stream";
    };

    match extension.to_ascii_lowercase().as_str() {
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "flv" => "video/x-flv",
        "mov" => "video/quicktime",
        "mpg" | "mpeg" => "video/mpeg",
        "ogv" => "video/ogg",
        "ts" => "video/mp2t",
        "wmv" => "video/x-ms-wmv",
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "ogg" => "audio/ogg",
        "opus" => "audio/opus",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "wma" => "audio/x-ms-wma",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "avif" => "image/avif",
        "ico" => "image/vnd.microsoft.icon",
        "vtt" => "text/vtt",
        "srt" | "txt" | "md" | "log" => "text/plain",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

#[cfg(any(feature = "api", test))]
pub(crate) fn media_kind(path: &Path) -> &'static str {
    let content_type = content_type(path);

    if content_type.starts_with("image/") {
        "image"
    } else if content_type.starts_with("video/") {
        "video"
    } else if content_type.starts_with("audio/") {
        "audio"
    } else {
        "text"
    }
}

#[cfg(test)]
mod tests {
    use super::{content_type, media_kind};
    use std::path::Path;

    #[test]
    fn maps_extensions_to_content_types() {
        assert_eq!(content_type(Path::new("movie.mp4")), "video/mp4");
        assert_eq!(content_type(Path::new("MOVIE.MKV")), "video/x-matroska");
        assert_eq!(content_type(Path::new("song.flac")), "audio/flac");
        assert_eq!(content_type(Path::new("photo.jpeg")), "image/jpeg");
        assert_eq!(
            content_type(Path::new("icon.ico")),
            "image/vnd.microsoft.icon"
        );
        assert_eq!(content_type(Path::new("movie.flv")), "video/x-flv");
        assert_eq!(content_type(Path::new("movie.ogv")), "video/ogg");
        assert_eq!(content_type(Path::new("movie.wmv")), "video/x-ms-wmv");
        assert_eq!(content_type(Path::new("song.wma")), "audio/x-ms-wma");
        assert_eq!(content_type(Path::new("notes.txt")), "text/plain");
        assert_eq!(
            content_type(Path::new("unknown.xyz")),
            "application/octet-stream"
        );
        assert_eq!(
            content_type(Path::new("no-extension")),
            "application/octet-stream"
        );
    }

    #[test]
    fn classifies_media_kinds() {
        for extension in [
            "avif", "bmp", "gif", "ico", "jpeg", "jpg", "png", "svg", "webp",
        ] {
            assert_eq!(media_kind(Path::new(&format!("file.{extension}"))), "image");
        }
        for extension in [
            "avi", "flv", "m4v", "mkv", "mov", "mp4", "mpeg", "mpg", "ogv", "ts", "webm", "wmv",
        ] {
            assert_eq!(media_kind(Path::new(&format!("file.{extension}"))), "video");
        }
        for extension in ["aac", "flac", "m4a", "mp3", "ogg", "opus", "wav", "wma"] {
            assert_eq!(media_kind(Path::new(&format!("file.{extension}"))), "audio");
        }

        assert_eq!(media_kind(Path::new(".JPG")), "image");
        assert_eq!(media_kind(Path::new("file.unknown")), "text");
        assert_eq!(media_kind(Path::new("no-extension")), "text");
    }
}
