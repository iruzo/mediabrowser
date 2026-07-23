use crate::response::{self, Response};
use hyper::http::StatusCode;
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};

const MAX_PATH_SIZE: usize = 4096;

pub async fn handle_ui(path: &str) -> Response {
    let Ok(path) = percent_decode_str(path).decode_utf8() else {
        return response::text(StatusCode::BAD_REQUEST, "path is not UTF-8");
    };

    if path.len() > MAX_PATH_SIZE || path.chars().any(|c| c.is_control() || c == '\\') {
        return response::text(StatusCode::BAD_REQUEST, "invalid path");
    }

    response::html(render_ui(&path))
}

fn render_ui(path: &str) -> String {
    let encoded = utf8_percent_encode(path, NON_ALPHANUMERIC).to_string();
    let mut title = String::from("/");
    escape_html(path, &mut title);

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>{title}</title>
<style>
html, body {{ margin: 0; height: 100%; }}
iframe {{ border: 0; width: 100%; height: 100%; display: block; }}
</style>
</head>
<body>
<iframe src="/grid?path={encoded}"></iframe>
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
    use super::render_ui;

    #[test]
    fn embeds_grid_iframe_with_encoded_path() {
        let html = render_ui("folder/sub dir");

        assert!(html.contains(r#"<iframe src="/grid?path=folder%2Fsub%20dir"></iframe>"#));
        assert!(html.contains("<title>/folder/sub dir</title>"));
    }

    #[test]
    fn escapes_html_in_title() {
        let html = render_ui("<a>&\"</a>");

        assert!(html.contains("<title>/&lt;a&gt;&amp;&quot;&lt;/a&gt;</title>"));
    }
}
