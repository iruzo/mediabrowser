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

pub(super) fn render_grid(path: &str, items: &[(String, bool)]) -> String {
    let mut boxes = String::with_capacity(items.len() * 96);

    for (index, (name, is_dir)) in items.iter().enumerate() {
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
        }
        boxes.push_str("</a>");
        // One form per item: each action picks its own endpoint, and the
        // closing button resets every control the menu left checked
        let menu = format!("menu-{index}");
        // rm takes two clicks: the checkbox hides its label and reveals the submit button
        let toggle = format!("rm-{index}");
        // Dismissing is a third click because no response comes back to act on
        let dismiss = format!("dismiss-{index}");
        boxes.push_str(
            r#"<form class="menu" method="post"><input type="hidden" name="path" value=""#,
        );
        escape_html(&child, &mut boxes);
        boxes.push_str(r#""><input type="checkbox" class="show" id=""#);
        boxes.push_str(&menu);
        boxes.push_str(r#""><label class="toggle" for=""#);
        boxes.push_str(&menu);
        boxes.push_str(
            r#"">...</label><button class="toggle" type="reset">...</button><div class="actions">"#,
        );
        boxes.push_str(
            r#"<button class="download" type="submit" formaction="/api/download">download</button>"#,
        );
        boxes.push_str(r#"<button class="cp" type="button">cp</button>"#);
        boxes.push_str(r#"<button class="mv" type="button">mv</button>"#);
        boxes.push_str(r#"<input type="checkbox" class="arm" id=""#);
        boxes.push_str(&toggle);
        boxes.push_str(r#""><label class="rm" for=""#);
        boxes.push_str(&toggle);
        boxes.push_str(r#"">rm</label><button class="confirm" type="submit" formaction="/api/rm" formtarget="rm">confirm</button><input type="checkbox" class="hide" id=""#);
        boxes.push_str(&dismiss);
        boxes.push_str(r#""><label class="dismiss" for=""#);
        boxes.push_str(&dismiss);
        boxes.push_str(r#"">hide item</label></div></form>"#);
        boxes.push_str("</div>\n");
    }

    let mut title = String::from("/");
    escape_html(path, &mut title);

    TEMPLATE
        .replace("{{TITLE}}", &title)
        .replace("{{CSS}}", CSS)
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
    fn renders_menu_buttons_alongside_the_open_link() {
        let items = vec![("file.txt".to_string(), false)];
        let html = render_grid("", &items);

        assert!(html.contains(r#"<div class="box"><a class="open" href="/file.txt" target="_top">file.txt</a><form class="menu" method="post">"#));
        assert!(html.contains(
            r#"<button class="cp" type="button">cp</button><button class="mv" type="button">mv</button>"#
        ));
    }

    #[test]
    fn arms_rm_with_a_checkbox_before_posting_to_the_endpoint() {
        let items = vec![("a.txt".to_string(), false), ("b.txt".to_string(), false)];
        let html = render_grid("root", &items);

        assert!(html.contains(
            r#"<input type="checkbox" class="arm" id="rm-0"><label class="rm" for="rm-0">rm</label><button class="confirm" type="submit" formaction="/api/rm" formtarget="rm">confirm</button><input type="checkbox" class="hide" id="dismiss-0"><label class="dismiss" for="dismiss-0">hide item</label>"#
        ));
        // Each box needs its own ids, otherwise one label acts on every item
        assert!(html.contains(r#"<input type="checkbox" class="arm" id="rm-1">"#));
        assert!(html.contains(r#"<input type="checkbox" class="hide" id="dismiss-1">"#));
        // The response goes to the iframe in the template, not to the page
        assert!(html.contains(r#"<iframe name="rm" hidden></iframe>"#));
    }

    #[test]
    fn closes_the_menu_with_a_reset_so_rm_does_not_stay_armed() {
        let items = vec![("a.txt".to_string(), false), ("b.txt".to_string(), false)];
        let html = render_grid("root", &items);

        assert!(html.contains(
            r#"<input type="checkbox" class="show" id="menu-0"><label class="toggle" for="menu-0">...</label><button class="toggle" type="reset">...</button>"#
        ));
        assert!(html.contains(r#"<input type="checkbox" class="show" id="menu-1">"#));
    }

    #[test]
    fn posts_the_item_path_to_the_download_endpoint() {
        let items = vec![
            ("a \"b\".txt".to_string(), false),
            ("sub".to_string(), true),
        ];
        let html = render_grid("root", &items);

        assert!(html.contains(
            r#"<form class="menu" method="post"><input type="hidden" name="path" value="root/sub"><input type="checkbox" class="show" id="menu-1">"#
        ));
        assert!(html.contains(
            r#"<button class="download" type="submit" formaction="/api/download">download</button>"#
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
