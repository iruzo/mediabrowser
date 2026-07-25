use percent_encoding::{AsciiSet, CONTROLS};
use std::path::Path;

// Encode only characters that are not allowed in URL paths (matching Apache)
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

pub(crate) fn content_type(path: &Path) -> &'static str {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return "application/octet-stream";
    };

    match extension.to_ascii_lowercase().as_str() {
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "mpg" | "mpeg" => "video/mpeg",
        "ts" => "video/mp2t",
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "ogg" => "audio/ogg",
        "opus" => "audio/opus",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "avif" => "image/avif",
        "vtt" => "text/vtt",
        "srt" | "txt" | "md" | "log" => "text/plain",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::content_type;
    use std::path::Path;

    #[test]
    fn maps_extensions_to_content_types() {
        assert_eq!(content_type(Path::new("movie.mp4")), "video/mp4");
        assert_eq!(content_type(Path::new("MOVIE.MKV")), "video/x-matroska");
        assert_eq!(content_type(Path::new("song.flac")), "audio/flac");
        assert_eq!(content_type(Path::new("photo.jpeg")), "image/jpeg");
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
}
