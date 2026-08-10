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

const TEMPLATE: &str = include_str!("grid.html");
const CSS: &str = include_str!("grid.css");
const SCRIPT: &str = include_str!("grid.js");

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

        boxes.push_str(r#"<div class="box">"#);
        boxes.push_str(r#"<a class="open" href=""#);
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
            // Only the label carries the slash; hrefs and form paths stay bare
            if *is_dir {
                boxes.push('/');
            }
        }
        boxes.push_str("</a>");
        boxes.push_str(r#"<details class="menu"><summary>...</summary><form class="actions" method="post" action="/api/rm"><input type="hidden" name="path" value=""#);
        escape_html(&child, &mut boxes);
        boxes.push_str(r#"">"#);
        boxes.push_str(
            r#"<button class="download" formaction="/api/download">download</button>"#,
        );
        boxes.push_str(r#"<input class="to" name="to" value=""#);
        escape_html(&child, &mut boxes);
        boxes.push_str(r#"" hidden><button class="mv" type="button">mv</button>"#);
        boxes.push_str(r#"<button class="rm" type="button">rm</button></form></details>"#);
        boxes.push_str("</div>\n");
    }

    let mut title = String::from("/");
    escape_html(path, &mut title);

    TEMPLATE
        .replace("{{TITLE}}", &title)
        .replace("{{CSS}}", CSS)
        .replace("{{SCRIPT}}", SCRIPT)
        .replace("{{BOXES}}", &boxes)
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

        assert!(html.contains(r#"href="/root/a%20b.txt" target="_top">a b.txt</a>"#));
        assert!(html.contains(r#"href="/ui/root/sub" target="_top">sub/</a>"#));
        // The slash is decoration, so it must not reach the path fields
        assert!(html.contains(r#"<input type="hidden" name="path" value="root/sub">"#));
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
    fn renders_item_actions_in_a_details_menu() {
        let items = vec![("file.txt".to_string(), false)];
        let html = render_grid("", &items);

        assert!(html.contains(
            r#"<div class="box"><a class="open" href="/file.txt" target="_top">file.txt</a><details class="menu"><summary>...</summary><form class="actions" method="post" action="/api/rm">"#
        ));
        assert!(!html.contains(r#"class="cp""#));
        assert!(html.contains(
            r#"<input class="to" name="to" value="file.txt" hidden><button class="mv" type="button">mv</button>"#
        ));
    }

    #[test]
    fn renders_rm_as_a_javascript_action() {
        let items = vec![("a.txt".to_string(), false), ("b.txt".to_string(), false)];
        let html = render_grid("root", &items);

        assert_eq!(
            html.matches(r#"<button class="rm" type="button">rm</button>"#)
                .count(),
            2
        );
        assert!(html.contains("await post(button.form.action"));
        assert!(!html.contains(r#"class="arm""#));
        assert!(!html.contains(r#"<iframe name="rm""#));
    }

    #[test]
    fn renders_mkdir_as_an_html_form() {
        let html = render_grid("root", &[]);

        assert!(html.contains(r#"<button class="show-mkdir">mkdir</button>"#));
        assert!(html.contains(
            r#"<form class="mkdir" method="post" action="/api/mkdir" hidden>"#
        ));
        assert!(html.contains(r#"<input name="path" placeholder="folder path" required"#));
        assert!(html.contains("await post(mkdir.action"));
    }

    #[test]
    fn renders_mv_as_a_javascript_action() {
        let html = render_grid("root", &[("a\".txt".to_string(), false)]);

        assert!(html.contains(r#"name="to" value="root/a&quot;.txt" hidden"#));
        assert!(html.contains(r#"<button class="mv" type="button">mv</button>"#));
        assert!(html.contains(r#"await post("/api/mv", { from:"#));
        assert!(html.contains("location.reload()"));
    }

    #[test]
    fn renders_native_details_menus_without_generated_ids() {
        let items = vec![("a.txt".to_string(), false), ("b.txt".to_string(), false)];
        let html = render_grid("root", &items);

        assert_eq!(html.matches(r#"<details class="menu">"#).count(), 2);
        assert_eq!(html.matches("<summary>...</summary>").count(), 2);
        assert!(!html.contains(r#"class="show""#));
        assert!(!html.contains(r#"class="toggle""#));
    }

    #[test]
    fn posts_the_item_path_to_the_download_endpoint() {
        let items = vec![
            ("a \"b\".txt".to_string(), false),
            ("sub".to_string(), true),
        ];
        let html = render_grid("root", &items);

        assert!(html.contains(
            r#"<form class="actions" method="post" action="/api/rm"><input type="hidden" name="path" value="root/sub">"#
        ));
        assert!(html.contains(
            r#"<button class="download" formaction="/api/download">download</button>"#
        ));
        assert!(html.contains(r#"value="root/a &quot;b&quot;.txt""#));
    }

    #[test]
    fn escapes_html_in_names() {
        let mut out = String::new();
        escape_html("<a>&\"b\"</a>", &mut out);
        assert_eq!(out, "&lt;a&gt;&amp;&quot;b&quot;&lt;/a&gt;");
    }
}
