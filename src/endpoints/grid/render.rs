use crate::mime::content_type;
use std::cmp::Ordering;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Sort {
    Name,
    Type,
    Date,
    Size,
}

impl Sort {
    pub(super) fn parse(value: &str) -> Self {
        match value {
            "type" => Self::Type,
            "date" => Self::Date,
            "size" => Self::Size,
            _ => Self::Name,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct GridItem {
    pub(super) name: String,
    pub(super) is_dir: bool,
    pub(super) kind: &'static str,
    pub(super) size: u64,
    pub(super) date: u64,
}

impl GridItem {
    pub(super) fn new(name: String, is_dir: bool, size: u64, modified: Option<SystemTime>) -> Self {
        Self {
            kind: media_kind(&name, is_dir),
            name,
            is_dir,
            size,
            date: modified
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |duration| duration.as_secs()),
        }
    }
}

pub(super) fn sort_items(items: &mut [GridItem], sort: Sort) {
    items.sort_by(|a, b| compare_items(a, b, sort));
}

fn compare_items(a: &GridItem, b: &GridItem, sort: Sort) -> Ordering {
    let dirs = b.is_dir.cmp(&a.is_dir);
    if dirs != Ordering::Equal {
        return dirs;
    }

    if a.is_dir {
        return compare_names(&a.name, &b.name);
    }

    let value = match sort {
        Sort::Name => Ordering::Equal,
        Sort::Type => a.kind.cmp(b.kind),
        Sort::Date => b.date.cmp(&a.date),
        Sort::Size => b.size.cmp(&a.size),
    };
    value.then_with(|| compare_names(&a.name, &b.name))
}

fn compare_names(a: &str, b: &str) -> Ordering {
    a.to_lowercase()
        .cmp(&b.to_lowercase())
        .then_with(|| a.cmp(b))
}

fn media_kind(name: &str, is_dir: bool) -> &'static str {
    if is_dir {
        return "dir";
    }

    let extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "bmp" | "gif" | "ico" | "jpeg" | "jpg" | "png" | "svg" | "webp" => "image",
        "avi" | "flv" | "m4v" | "mkv" | "mov" | "mp4" | "ogv" | "webm" | "wmv" => "video",
        "aac" | "flac" | "m4a" | "mp3" | "ogg" | "wav" | "wma" => "audio",
        _ => "text",
    }
}

const TEMPLATE: &str = include_str!("grid.html");
const CSS: &str = include_str!("grid.css");
const SCRIPT: &str = include_str!("grid.js");

pub(super) fn render_grid(path: &str, items: &[GridItem]) -> String {
    let mut rendered = String::with_capacity(items.len() * 320);

    for item in items {
        let child = if path.is_empty() {
            item.name.clone()
        } else {
            format!("{path}/{}", item.name)
        };
        let encoded = encode_path(&child);
        let href = if item.is_dir {
            format!("/ui/{encoded}")
        } else if item.kind == "text" {
            format!("/{encoded}")
        } else {
            let page = if path.is_empty() {
                "/ui".to_string()
            } else {
                format!("/ui/{}", encode_path(path))
            };
            format!("{page}?view={}", encode_component(&child))
        };

        rendered.push_str(r#"<div class="item "#);
        rendered.push_str(item.kind);
        rendered.push_str(r#"" data-path=""#);
        escape_html(&child, &mut rendered);
        rendered.push_str(r#"" data-kind=""#);
        rendered.push_str(item.kind);
        rendered.push_str(r#"" data-size=""#);
        rendered.push_str(&item.size.to_string());
        rendered.push_str(r#"" data-date=""#);
        rendered.push_str(&item.date.to_string());
        rendered.push_str(r#""><a href=""#);
        rendered.push_str(&href);
        rendered.push_str(r#"" target="_top">"#);

        if !item.is_dir && content_type(Path::new(&item.name)).starts_with("image/") {
            rendered.push_str(r#"<img src=""#);
            rendered.push_str(&format!("/{encoded}"));
            rendered.push_str(r#"" loading="lazy" alt=""#);
            escape_html(&item.name, &mut rendered);
            rendered.push_str(r#"">"#);
        }

        rendered.push_str(r#"<span class="name">"#);
        escape_html(&item.name, &mut rendered);
        if item.is_dir {
            rendered.push('/');
        }
        rendered.push_str("</span></a>");
        rendered.push_str(r#"<details class="menu"><summary>...</summary><form class="actions"><button class="download" type="button">download</button><input class="to cp-to" value=""#);
        escape_html(&child, &mut rendered);
        rendered.push_str(r#"" hidden><button class="cp" type="button">cp</button><input class="to mv-to" value=""#);
        escape_html(&child, &mut rendered);
        rendered.push_str(r#"" hidden><button class="mv" type="button">mv</button><button class="rm" type="button">rm</button></form></details></div>
"#);
    }

    let mut title = String::from("/");
    escape_html(path, &mut title);
    let mut escaped_path = String::new();
    escape_html(path, &mut escaped_path);

    TEMPLATE
        .replace("{{TITLE}}", &title)
        .replace("{{PATH}}", &escaped_path)
        .replace("{{CSS}}", CSS)
        .replace("GRID_SCRIPT", SCRIPT)
        .replace("{{BOXES}}", &rendered)
}

fn encode_path(path: &str) -> String {
    path.split('/')
        .map(encode_component)
        .collect::<Vec<_>>()
        .join("/")
}

fn encode_component(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            )
        {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}

fn escape_html(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{escape_html, media_kind, render_grid, sort_items, GridItem, Sort};
    use std::time::{Duration, UNIX_EPOCH};

    fn item(name: &str, is_dir: bool, size: u64, date: Option<u64>) -> GridItem {
        GridItem::new(
            name.to_string(),
            is_dir,
            size,
            date.map(|seconds| UNIX_EPOCH + Duration::from_secs(seconds)),
        )
    }

    fn names(items: &[GridItem]) -> Vec<&str> {
        items.iter().map(|item| item.name.as_str()).collect()
    }

    #[test]
    fn sorts_names_case_insensitively_with_directories_first() {
        let mut items = vec![
            item("zzz.txt", false, 0, None),
            item("beta", true, 0, None),
            item("Alpha", true, 0, None),
            item("AAA.txt", false, 0, None),
        ];
        sort_items(&mut items, Sort::Name);
        assert_eq!(names(&items), ["Alpha", "beta", "AAA.txt", "zzz.txt"]);
    }

    #[test]
    fn sorts_types_then_names_with_directories_first() {
        let mut items = vec![
            item("z.txt", false, 1, Some(1)),
            item("v.mp4", false, 1, Some(1)),
            item("a.mp3", false, 1, Some(1)),
            item("i.jpg", false, 1, Some(1)),
            item("Dir", true, 1, Some(1)),
        ];
        sort_items(&mut items, Sort::Type);
        assert_eq!(names(&items), ["Dir", "a.mp3", "i.jpg", "z.txt", "v.mp4"]);
    }

    #[test]
    fn sorts_newest_and_largest_first_with_name_ties() {
        let source = vec![
            item("missing.txt", false, 0, None),
            item("b.txt", false, 20, Some(5)),
            item("A.txt", false, 20, Some(5)),
            item("old.txt", false, 10, Some(2)),
            item("z-dir", true, 99, Some(99)),
            item("a-dir", true, 0, None),
        ];
        let mut dates = source.clone();
        sort_items(&mut dates, Sort::Date);
        assert_eq!(
            names(&dates),
            ["a-dir", "z-dir", "A.txt", "b.txt", "old.txt", "missing.txt"]
        );

        let mut sizes = source;
        sort_items(&mut sizes, Sort::Size);
        assert_eq!(
            names(&sizes),
            ["a-dir", "z-dir", "A.txt", "b.txt", "old.txt", "missing.txt"]
        );
    }

    #[test]
    fn classifies_the_existing_client_extension_sets() {
        assert_eq!(media_kind("PHOTO.ICO", false), "image");
        assert_eq!(media_kind("clip.flv", false), "video");
        assert_eq!(media_kind("sound.wma", false), "audio");
        assert_eq!(media_kind("photo.avif", false), "text");
        assert_eq!(media_kind("movie.mpeg", false), "text");
        assert_eq!(media_kind("folder.jpg", true), "dir");
    }

    #[test]
    fn parses_all_sort_modes() {
        assert_eq!(Sort::parse("name"), Sort::Name);
        assert_eq!(Sort::parse("type"), Sort::Type);
        assert_eq!(Sort::parse("date"), Sort::Date);
        assert_eq!(Sort::parse("size"), Sort::Size);
        assert_eq!(Sort::parse("unknown"), Sort::Name);
    }

    #[test]
    fn renders_final_items_links_actions_and_metadata() {
        let items = vec![
            item("a & b.txt", false, 12, Some(34)),
            item("photo.JPG", false, 56, Some(78)),
            item("sub", true, 0, None),
        ];
        let html = render_grid("root", &items);

        assert!(html.contains(r#"class="item text" data-path="root/a &amp; b.txt" data-kind="text" data-size="12" data-date="34""#));
        assert!(html.contains(
            r#"href="/root/a%20%26%20b.txt" target="_top"><span class="name">a &amp; b.txt</span>"#
        ));
        assert!(html.contains(r#"class="item image" data-path="root/photo.JPG" data-kind="image" data-size="56" data-date="78""#));
        assert!(html.contains(r#"<img src="/root/photo.JPG" loading="lazy" alt="photo.JPG"><span class="name">photo.JPG</span>"#));
        assert!(
            html.contains(r#"href="/ui/root/sub" target="_top"><span class="name">sub/</span>"#)
        );
        assert_eq!(html.matches(r#"<details class="menu">"#).count(), 3);
        assert!(html.contains(r#"<button class="download" type="button">download</button>"#));
        assert!(html.contains(r#"class="to cp-to" value="root/a &amp; b.txt" hidden"#));
        assert!(html.contains(r#"class="to mv-to" value="root/a &amp; b.txt" hidden"#));
    }

    #[test]
    fn preserves_uncommon_thumbnail_and_viewer_behavior() {
        let html = render_grid(
            "",
            &[
                item("icon.ico", false, 0, None),
                item("new.avif", false, 0, None),
            ],
        );
        assert!(html.contains(r#"class="item image" data-path="icon.ico""#));
        assert!(!html.contains(r#"<img src="/icon.ico""#));
        assert!(html.contains(r#"class="item text" data-path="new.avif""#));
        assert!(html.contains(r#"<img src="/new.avif""#));
    }

    #[test]
    fn escapes_html() {
        let mut out = String::new();
        escape_html("<a>&\"b\"</a>", &mut out);
        assert_eq!(out, "&lt;a&gt;&amp;&quot;b&quot;&lt;/a&gt;");
    }

    #[test]
    fn leaves_no_transitional_markup_or_script_marker() {
        let html = render_grid("", &[item("file.txt", false, 0, None)]);
        assert!(!html.contains(r#"class="box""#));
        assert!(!html.contains("makeItem"));
        assert!(!html.contains("GRID_SCRIPT"));
    }
}
