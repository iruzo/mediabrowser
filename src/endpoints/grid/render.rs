use crate::mime::{content_type, PATH_SEGMENT};
use percent_encoding::utf8_percent_encode;
use std::path::Path;

pub(super) fn sort_items(items: &mut [(String, bool)]) {
    items.sort_by(|(a_name, a_is_dir), (b_name, b_is_dir)| {
        b_is_dir
            .cmp(a_is_dir)
            .then_with(|| a_name.to_lowercase().cmp(&b_name.to_lowercase()))
    });
}

pub(super) fn render_grid(path: &str, items: &[(String, bool)]) -> String {
    let mut boxes = String::with_capacity(items.len() * 96);

    for (name, is_dir) in items {
        let child = if path.is_empty() {
            name.clone()
        } else {
            format!("{path}/{name}")
        };
        let encoded = utf8_percent_encode(&child, PATH_SEGMENT).to_string();
        let href = if *is_dir {
            format!("/ui/{encoded}")
        } else {
            format!("/{encoded}")
        };

        boxes.push_str(r#"<a class="box" href=""#);
        boxes.push_str(&href);
        boxes.push_str(r#"" target="_top">"#);
        if !is_dir && content_type(Path::new(name)).starts_with("image/") {
            boxes.push_str(r#"<img src=""#);
            boxes.push_str(&href);
            boxes.push_str(r#"" loading="lazy" alt=""#);
            escape_html(name, &mut boxes);
            boxes.push_str(r#"">"#);
        } else {
            escape_html(name, &mut boxes);
        }
        boxes.push_str("</a>\n");
    }

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
body {{ font-family: sans-serif; margin: 0; background: #111; color: #eee; }}
.grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); }}
.box {{ display: flex; align-items: center; justify-content: center; height: 120px;
  text-align: center; word-break: break-all; text-decoration: none; color: inherit;
  background: #222; overflow: hidden; }}
.box img {{ width: 100%; height: 100%; object-fit: cover; display: block; }}
</style>
</head>
<body>
<div class="grid">
{boxes}</div>
</body>
</html>"#
    )
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
    use super::{escape_html, render_grid, sort_items};

    #[test]
    fn sorts_dirs_before_files_regardless_of_name() {
        let mut items = vec![
            ("zzz-file.txt".to_string(), false),
            ("aaa-dir".to_string(), true),
            ("aaa-file.txt".to_string(), false),
            ("zzz-dir".to_string(), true),
        ];
        sort_items(&mut items);

        assert_eq!(
            items,
            vec![
                ("aaa-dir".to_string(), true),
                ("zzz-dir".to_string(), true),
                ("aaa-file.txt".to_string(), false),
                ("zzz-file.txt".to_string(), false),
            ]
        );
    }

    #[test]
    fn renders_links_for_files_and_dirs() {
        let items = vec![("a b.txt".to_string(), false), ("sub".to_string(), true)];
        let html = render_grid("root", &items);

        assert!(html.contains(r#"href="/root/a%20b.txt" target="_top""#));
        assert!(html.contains(r#"href="/ui/root/sub" target="_top""#));
    }

    #[test]
    fn renders_img_tag_for_images_and_gifs_but_not_other_files() {
        let items = vec![
            ("photo.JPG".to_string(), false),
            ("anim.gif".to_string(), false),
            ("notes.txt".to_string(), false),
            ("sub".to_string(), true),
        ];
        let html = render_grid("", &items);

        assert!(html.contains(
            r#"href="/photo.JPG" target="_top"><img src="/photo.JPG" loading="lazy" alt="photo.JPG"></a>"#
        ));
        assert!(html.contains(
            r#"href="/anim.gif" target="_top"><img src="/anim.gif" loading="lazy" alt="anim.gif"></a>"#
        ));
        assert!(!html.contains(r#"<img src="/notes.txt""#));
        assert!(!html.contains(r#"<img src="/ui/sub""#));
    }

    #[test]
    fn escapes_html_in_names() {
        let mut out = String::new();
        escape_html("<a>&\"b\"</a>", &mut out);
        assert_eq!(out, "&lt;a&gt;&amp;&quot;b&quot;&lt;/a&gt;");
    }
}
