use crate::endpoints::delete::handle_delete;
use crate::endpoints::download_bulk::{handle_downloads, DownloadBulkRequest};
use crate::endpoints::mkdir::handle_mkdir;
use crate::endpoints::mv::{handle_mv, MvItem};
use crate::endpoints::save::handle_save;
use crate::types::{api_path, data_path, FileQuery};
use percent_encoding::{
    percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS, NON_ALPHANUMERIC,
};
use serde::Deserialize;
use std::convert::Infallible;
use std::fmt::Write as _;
use std::path::Path;
use std::time::UNIX_EPOCH;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use walkdir::WalkDir;
use warp::http::StatusCode;
use warp::hyper::Body;
use warp::{Filter, Reply};

const EDITOR_MAX_SIZE: u64 = 512 * 1024;
const CHUNK_SIZE: u64 = 64 * 1024;
const MAX_SEARCH_RESULTS: usize = 500;
const FORM_MAX_SIZE: u64 = 16 * 1024 * 1024;

const SORTS: [&str; 4] = ["name", "date", "size", "type"];
const FILTERS: [&str; 5] = ["all", "image", "video", "audio", "text"];

const IMAGE_EXTS: [&str; 8] = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "svg", "ico"];
const VIDEO_EXTS: [&str; 9] = [
    "mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "m4v", "ogv",
];
const AUDIO_EXTS: [&str; 7] = ["mp3", "wav", "flac", "aac", "ogg", "wma", "m4a"];

// Characters percent-encoded in hrefs so generated URLs are safe inside
// double-quoted HTML attributes without further escaping
const HREF_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'\'')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'#')
    .add(b'?')
    .add(b'{')
    .add(b'}')
    .add(b'%')
    .add(b'&');

#[derive(Deserialize)]
pub struct RenderQuery {
    q: Option<String>,
    sort: Option<String>,
    filter: Option<String>,
    offset: Option<u64>,
}

struct Item {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: u64,
}

struct View {
    sort: String,
    filter: String,
    q: String,
}

impl View {
    fn suffix(&self, with_q: bool) -> String {
        query_suffix(&self.sort, &self.filter, if with_q { &self.q } else { "" })
    }
}

enum ViewerContent {
    Image,
    Video,
    Audio,
    Editor {
        text: String,
    },
    Chunk {
        text: String,
        offset: u64,
        end: u64,
        size: u64,
    },
}

pub fn render_routes() -> warp::filters::BoxedFilter<(warp::reply::Response,)> {
    let forms = warp::path!("ui" / "form" / String)
        .and(warp::post())
        .and(warp::body::content_length_limit(FORM_MAX_SIZE))
        .and(warp::body::bytes())
        .and_then(handle_form);

    let index = warp::path("ui")
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<RenderQuery>())
        .and_then(|query| handle_page(String::new(), query));

    let pages = warp::path("ui")
        .and(warp::get())
        .and(warp::path::tail())
        .and(warp::query::<RenderQuery>())
        .and_then(|tail: warp::path::Tail, query| handle_page(tail.as_str().to_string(), query));

    forms.or(index).unify().or(pages).unify().boxed()
}

// Page handling

async fn handle_page(
    tail: String,
    query: RenderQuery,
) -> Result<warp::reply::Response, Infallible> {
    let decoded = percent_decode_str(tail.trim_matches('/'))
        .decode_utf8_lossy()
        .into_owned();
    let view = make_view(&query);

    let Some(fs_path) = data_path(&decoded) else {
        return Ok(
            warp::reply::with_status("Access denied", StatusCode::FORBIDDEN).into_response(),
        );
    };

    let Ok(metadata) = fs::metadata(&fs_path).await else {
        return Ok(html_response(render_not_found(), StatusCode::NOT_FOUND));
    };

    if metadata.is_dir() {
        let mut items = if view.q.is_empty() {
            read_items(&fs_path).await
        } else {
            search_items(&fs_path, &view.q).await
        };

        filter_items(&mut items, &view.filter);
        sort_items(&mut items, &view.sort);

        return Ok(html_response(
            render_gallery(&decoded, &items, &view),
            StatusCode::OK,
        ));
    }

    let name = decoded.rsplit('/').next().unwrap_or(&decoded).to_string();
    let content = match file_kind(&name) {
        "image" => ViewerContent::Image,
        "video" => ViewerContent::Video,
        "audio" => ViewerContent::Audio,
        _ => {
            if metadata.len() <= EDITOR_MAX_SIZE {
                let bytes = fs::read(&fs_path).await.unwrap_or_default();
                ViewerContent::Editor {
                    text: String::from_utf8_lossy(&bytes).into_owned(),
                }
            } else {
                let offset = query.offset.unwrap_or(0).min(metadata.len());
                let (text, read) = read_chunk(&fs_path, offset).await;
                ViewerContent::Chunk {
                    text,
                    offset,
                    end: offset + read,
                    size: metadata.len(),
                }
            }
        }
    };

    let (prev, next) = sibling_files(&decoded, &view).await;

    Ok(html_response(
        render_viewer(&decoded, &name, &content, &prev, &next, &view),
        StatusCode::OK,
    ))
}

fn make_view(query: &RenderQuery) -> View {
    let sort = query.sort.clone().unwrap_or_default();
    let filter = query.filter.clone().unwrap_or_default();

    View {
        sort: if SORTS.contains(&sort.as_str()) {
            sort
        } else {
            "name".to_string()
        },
        filter: if FILTERS.contains(&filter.as_str()) {
            filter
        } else {
            "all".to_string()
        },
        q: query.q.clone().unwrap_or_default().trim().to_string(),
    }
}

async fn sibling_files(path: &str, view: &View) -> (String, String) {
    let dir = path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    let fallback = (path.to_string(), path.to_string());

    let Some(dir_fs) = data_path(dir) else {
        return fallback;
    };

    let mut files = read_items(&dir_fs).await;
    files.retain(|item| !item.is_dir);
    filter_items(&mut files, &view.filter);
    sort_items(&mut files, &view.sort);

    let Some(index) = files.iter().position(|item| item.path == path) else {
        return fallback;
    };

    let prev = files[(index + files.len() - 1) % files.len()].path.clone();
    let next = files[(index + 1) % files.len()].path.clone();
    (prev, next)
}

async fn read_chunk(path: &Path, offset: u64) -> (String, u64) {
    let Ok(mut file) = fs::File::open(path).await else {
        return (String::new(), 0);
    };

    if file.seek(std::io::SeekFrom::Start(offset)).await.is_err() {
        return (String::new(), 0);
    }

    let mut buffer = Vec::with_capacity(CHUNK_SIZE as usize);
    if file
        .take(CHUNK_SIZE)
        .read_to_end(&mut buffer)
        .await
        .is_err()
    {
        return (String::new(), 0);
    }

    let read = buffer.len() as u64;
    (String::from_utf8_lossy(&buffer).into_owned(), read)
}

// Directory listing and search

fn modified_millis(metadata: &std::fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

// The scans run as one blocking task: per-entry async stat calls would pay a
// thread pool round-trip each, and blocking the async workers would stall
// other connections
async fn read_items(dir_path: &Path) -> Vec<Item> {
    let dir_path = dir_path.to_path_buf();
    tokio::task::spawn_blocking(move || read_items_sync(&dir_path))
        .await
        .unwrap_or_default()
}

fn read_items_sync(dir_path: &Path) -> Vec<Item> {
    let mut items = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir_path) else {
        return items;
    };

    for entry in entries.flatten() {
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        let name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(_) => continue,
        };

        items.push(Item {
            path: api_path(&entry.path()),
            name,
            is_dir: metadata.is_dir(),
            size: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
            modified: modified_millis(&metadata),
        });
    }

    items
}

async fn search_items(dir_path: &Path, search: &str) -> Vec<Item> {
    let dir_path = dir_path.to_path_buf();
    let search = search.to_string();
    tokio::task::spawn_blocking(move || search_items_sync(&dir_path, &search))
        .await
        .unwrap_or_default()
}

fn search_items_sync(dir_path: &Path, search: &str) -> Vec<Item> {
    let terms: Vec<String> = search
        .split_whitespace()
        .map(|term| term.to_lowercase())
        .filter(|term| !term.is_empty())
        .collect();

    let mut items = Vec::new();
    if terms.is_empty() {
        return items;
    }

    for entry in WalkDir::new(dir_path)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        let relative = path
            .strip_prefix(dir_path)
            .ok()
            .map(|value| value.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if !terms.iter().all(|term| relative.contains(term)) {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        items.push(Item {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: api_path(path),
            is_dir: metadata.is_dir(),
            size: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
            modified: modified_millis(&metadata),
        });

        if items.len() >= MAX_SEARCH_RESULTS {
            break;
        }
    }

    items
}

fn filter_items(items: &mut Vec<Item>, filter: &str) {
    if filter == "all" {
        return;
    }

    items.retain(|item| item.is_dir || file_kind(&item.name) == filter);
}

fn sort_items(items: &mut [Item], sort: &str) {
    items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => match sort {
            "date" => b.modified.cmp(&a.modified),
            "size" => b.size.cmp(&a.size),
            "type" => file_kind(&a.name)
                .cmp(file_kind(&b.name))
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        },
    });
}

fn file_kind(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();

    if IMAGE_EXTS.contains(&ext.as_str()) {
        "image"
    } else if VIDEO_EXTS.contains(&ext.as_str()) {
        "video"
    } else if AUDIO_EXTS.contains(&ext.as_str()) {
        "audio"
    } else {
        "text"
    }
}

// Form handling

async fn handle_form(
    action: String,
    body: bytes::Bytes,
) -> Result<warp::reply::Response, Infallible> {
    let fields = parse_form(&body);
    let paths: Vec<String> = fields
        .iter()
        .filter(|(key, _)| key == "paths")
        .map(|(_, value)| value.clone())
        .collect();

    match action.as_str() {
        "download" => return handle_downloads(DownloadBulkRequest { paths }).await,
        "save" => {
            if let Some(path) = field(&fields, "path") {
                let query = FileQuery {
                    path: path.to_string(),
                };
                let body = bytes::Bytes::copy_from_slice(
                    field(&fields, "content").unwrap_or_default().as_bytes(),
                );
                let _ = handle_save(query, body).await;
            }
        }
        "mkdir" => {
            let dir = field(&fields, "dir").unwrap_or_default();
            let name = field(&fields, "name").unwrap_or_default();
            let name = name.trim();
            if !name.is_empty() {
                let rel = if dir.is_empty() {
                    name.to_string()
                } else {
                    format!("{}/{}", dir, name)
                };
                let _ = handle_mkdir(FileQuery { path: rel }).await;
            }
        }
        "rename" => {
            let name = field(&fields, "name").unwrap_or_default();
            let items = rename_items(&paths, name.trim());
            if !items.is_empty() {
                let _ = handle_mv(items).await;
            }
        }
        "delete" => {
            for path in paths {
                let _ = handle_delete(FileQuery { path }).await;
            }
        }
        _ => {
            return Ok(
                warp::reply::with_status("Unknown form action", StatusCode::NOT_FOUND)
                    .into_response(),
            );
        }
    }

    Ok(redirect_response(&back_target(&fields)))
}

fn rename_items(paths: &[String], name: &str) -> Vec<MvItem> {
    if name.is_empty() {
        return Vec::new();
    }

    let multiple = paths.len() > 1;
    let mut items = Vec::with_capacity(paths.len());

    for (index, path) in paths.iter().enumerate() {
        let Some(source) = data_path(path) else {
            continue;
        };

        let (dir, old_name) = path.rsplit_once('/').unwrap_or(("", path));

        // Multi-rename appends an index and keeps file extensions
        let new_name = if multiple {
            let ext = if source.is_dir() {
                ""
            } else {
                old_name
                    .rfind('.')
                    .filter(|dot| *dot > 0)
                    .map(|dot| &old_name[dot..])
                    .unwrap_or("")
            };
            format!("{}{}{}", name, index + 1, ext)
        } else {
            name.to_string()
        };

        let rel = if dir.is_empty() {
            new_name
        } else {
            format!("{}/{}", dir, new_name)
        };

        items.push(MvItem {
            from: path.clone(),
            to: rel,
        });
    }

    items
}

fn parse_form(body: &[u8]) -> Vec<(String, String)> {
    std::str::from_utf8(body)
        .unwrap_or("")
        .split('&')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            Some((form_decode(key), form_decode(value)))
        })
        .collect()
}

fn form_decode(value: &str) -> String {
    let replaced = value.replace('+', " ");
    percent_decode_str(&replaced)
        .decode_utf8_lossy()
        .into_owned()
}

fn field<'a>(fields: &'a [(String, String)], name: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn back_target(fields: &[(String, String)]) -> String {
    field(fields, "back")
        .filter(|back| back.starts_with("/ui") && back.is_ascii())
        .unwrap_or("/ui/")
        .to_string()
}

fn redirect_response(location: &str) -> warp::reply::Response {
    warp::http::Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header("location", location)
        .body(Body::empty())
        .unwrap()
}

fn html_response(html: String, status: StatusCode) -> warp::reply::Response {
    warp::http::Response::builder()
        .status(status)
        .header("content-type", "text/html; charset=utf-8")
        .body(Body::from(html))
        .unwrap()
}

// HTML rendering

fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());

    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }

    out
}

fn href_path(path: &str) -> String {
    utf8_percent_encode(path, HREF_SET).to_string()
}

fn query_suffix(sort: &str, filter: &str, q: &str) -> String {
    let mut parts = Vec::new();

    if sort != "name" {
        parts.push(format!("sort={}", sort));
    }
    if filter != "all" {
        parts.push(format!("filter={}", filter));
    }
    if !q.is_empty() {
        parts.push(format!("q={}", utf8_percent_encode(q, NON_ALPHANUMERIC)));
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!("?{}", parts.join("&"))
    }
}

fn page(title: &str, body: &str) -> String {
    format!(
        r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0, interactive-widget=resizes-content">
<title>{}</title>
<link rel="stylesheet" href="/ui/assets/style.css">
<script defer src="/ui/assets/main.js"></script>
</head>
<body>
{}</body>
</html>
"#,
        html_escape(title),
        body
    )
}

fn render_gallery(dir: &str, items: &[Item], view: &View) -> String {
    let dir_href = if dir.is_empty() {
        "/ui/".to_string()
    } else {
        format!("/ui/{}/", href_path(dir))
    };
    let back = format!("{}{}", dir_href, view.suffix(true));
    let suffix = view.suffix(false);

    let mut grid = String::with_capacity(items.len() * 160);
    for item in items {
        let kind = if item.is_dir {
            "dir"
        } else {
            file_kind(&item.name)
        };

        let href = if item.is_dir {
            format!("/ui/{}/{}", href_path(&item.path), suffix)
        } else {
            format!("/ui/{}{}", href_path(&item.path), suffix)
        };

        let media = if kind == "image" {
            format!(
                r#"<img loading="lazy" src="/{}" alt="">"#,
                href_path(&item.path)
            )
        } else {
            String::new()
        };

        // Search results are shown with their full relative path
        let mut label = if view.q.is_empty() {
            item.name.clone()
        } else {
            item.path.clone()
        };
        if item.is_dir {
            label.push('/');
        }

        let _ = write!(
            grid,
            r#"<div class="item {}">
<a href="{}">{}<span class="name">{}</span></a>
<input form="grid" type="checkbox" name="paths" value="{}">
</div>
"#,
            kind,
            href,
            media,
            html_escape(&label),
            html_escape(&item.path)
        );
    }

    let mut search_hidden = String::new();
    if view.sort != "name" {
        let _ = write!(
            search_hidden,
            r#"<input type="hidden" name="sort" value="{}">"#,
            view.sort
        );
    }
    if view.filter != "all" {
        let _ = write!(
            search_hidden,
            r#"<input type="hidden" name="filter" value="{}">"#,
            view.filter
        );
    }

    let mut sort_links = String::new();
    for sort in SORTS {
        let on = if sort == view.sort {
            r#" class="on""#
        } else {
            ""
        };
        let _ = write!(
            sort_links,
            r#"<a{} href="{}{}">{}</a>"#,
            on,
            dir_href,
            query_suffix(sort, &view.filter, &view.q),
            sort
        );
    }

    let mut filter_links = String::new();
    for filter in FILTERS {
        let on = if filter == view.filter {
            r#" class="on""#
        } else {
            ""
        };
        let _ = write!(
            filter_links,
            r#"<a{} href="{}{}">{}</a>"#,
            on,
            dir_href,
            query_suffix(&view.sort, filter, &view.q),
            filter
        );
    }

    let title = format!("/{}", dir);
    let body = format!(
        r#"<main class="grid">
{grid}</main>
<div class="bar">
<form class="search" method="get" action="">
<input type="search" name="q" value="{q}" placeholder="search">
{search_hidden}
</form>
<button type="button" class="menu-toggle" popovertarget="menu">menu</button>
</div>
<div id="menu" popover>
<button type="button" id="upload">upload</button>
<input form="grid" name="name" placeholder="name">
<button form="grid" formaction="/ui/form/mkdir">create folder</button>
<button form="grid" formaction="/ui/form/rename">rename selected</button>
<button form="grid" formaction="/ui/form/download">download selected</button>
<button form="grid" formaction="/ui/form/delete" onclick="return confirm('delete selected?')">delete selected</button>
<button type="button" id="selectall">select all</button>
<div class="menu-row"><span>size</span><button type="button" id="cellminus">-</button><button type="button" id="cellplus">+</button></div>
<div class="menu-row"><span>sort</span>{sort_links}</div>
<div class="menu-row"><span>filter</span>{filter_links}</div>
</div>
<form id="grid" method="post" action="/ui/form/mkdir" hidden>
<input type="hidden" name="dir" value="{dir}">
<input type="hidden" name="back" value="{back}">
</form>
<input type="file" id="file" multiple hidden>
<output id="progress" hidden></output>
"#,
        grid = grid,
        q = html_escape(&view.q),
        search_hidden = search_hidden,
        sort_links = sort_links,
        filter_links = filter_links,
        dir = html_escape(dir),
        back = back,
    );

    page(&title, &body)
}

fn render_viewer(
    path: &str,
    name: &str,
    content: &ViewerContent,
    prev: &str,
    next: &str,
    view: &View,
) -> String {
    let enc = href_path(path);
    let suffix = view.suffix(false);
    let self_href = format!("/ui/{}{}", enc, suffix);
    let dir = path.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    let close = if dir.is_empty() {
        format!("/ui/{}", suffix)
    } else {
        format!("/ui/{}/{}", href_path(dir), suffix)
    };
    let prev_href = format!("/ui/{}{}", href_path(prev), suffix);
    let next_href = format!("/ui/{}{}", href_path(next), suffix);

    let main = match content {
        ViewerContent::Image => format!(
            r#"<img id="media" src="/{}" alt="{}">"#,
            enc,
            html_escape(name)
        ),
        ViewerContent::Video => format!(r#"<video id="media" src="/{}" controls></video>"#, enc),
        ViewerContent::Audio => format!(r#"<audio src="/{}" controls></audio>"#, enc),
        ViewerContent::Editor { text } => format!(
            r#"<form id="editor" class="editor" method="post" action="/ui/form/save" onsubmit="return confirm('save changes?')">
<input type="hidden" name="path" value="{}">
<input type="hidden" name="back" value="{}">
<textarea name="content">{}</textarea>
</form>"#,
            html_escape(path),
            self_href,
            html_escape(text)
        ),
        ViewerContent::Chunk {
            text,
            offset,
            end,
            size,
        } => {
            let chunk_href = |target: u64| {
                if target == 0 {
                    self_href.clone()
                } else if suffix.is_empty() {
                    format!("{}?offset={}", self_href, target)
                } else {
                    format!("{}&offset={}", self_href, target)
                }
            };

            let mut nav = String::new();
            if *offset > 0 {
                let _ = write!(
                    nav,
                    r#"<a href="{}">prev</a>"#,
                    chunk_href(offset.saturating_sub(CHUNK_SIZE))
                );
            }
            let _ = write!(nav, "<span>{}-{}/{} bytes</span>", offset, end, size);
            if *end < *size {
                let _ = write!(nav, r#"<a href="{}">next</a>"#, chunk_href(*end));
            }

            format!(
                r#"<div class="chunk">
<pre>{}</pre>
<nav class="chunknav">{}</nav>
</div>"#,
                html_escape(text),
                nav
            )
        }
    };

    let mut menu = format!(r#"<a href="/api/download/{}" download>download</a>"#, enc);
    if matches!(content, ViewerContent::Image | ViewerContent::Video) {
        menu.push_str(
            r#"<div class="menu-row"><span>zoom</span><button type="button" data-zoom="in">+</button><button type="button" data-zoom="out">-</button><button type="button" data-zoom="reset">reset</button></div>
<div class="menu-row"><span>move</span><button type="button" data-move="0 25">up</button><button type="button" data-move="0 -25">down</button><button type="button" data-move="25 0">left</button><button type="button" data-move="-25 0">right</button></div>
<div class="menu-row"><span>rotate</span><button type="button" data-rot="-45">left</button><button type="button" data-rot="45">right</button></div>"#,
        );
    }
    if matches!(content, ViewerContent::Video) {
        menu.push_str(
            r#"<div class="menu-row"><span>loop</span><input id="loopstart" placeholder="0:00"><span>-</span><input id="loopend" placeholder="0:00"><button type="button" id="loopclear">clear</button></div>
<div class="menu-row"><span>seek</span><button type="button" data-seek="-1">-1s</button><button type="button" data-seek="1">+1s</button></div>"#,
        );
    }
    if matches!(content, ViewerContent::Editor { .. }) {
        menu.push_str(r#"<button form="editor">save</button>"#);
    }

    let body = format!(
        r#"<main class="viewer">
{main}
</main>
<nav class="bar">
<a id="prev" href="{prev_href}">prev</a>
<a id="close" href="{close}">close</a>
<a id="next" href="{next_href}">next</a>
<button type="button" class="menu-toggle" popovertarget="menu">menu</button>
</nav>
<div id="menu" popover>
{menu}
</div>
"#,
        main = main,
        prev_href = prev_href,
        close = close,
        next_href = next_href,
        menu = menu,
    );

    page(name, &body)
}

fn render_not_found() -> String {
    page(
        "not found",
        r#"<main class="message">not found - <a href="/ui/">back</a></main>"#,
    )
}
