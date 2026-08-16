use crate::response::{self, Response};

const TEMPLATE: &str = include_str!("ui/index.html");
const CSS: &str = include_str!("ui/style.css");
const SCRIPT: &str = include_str!("ui/script.js");

pub fn handle_ui() -> Response {
    response::html(page())
}

fn page() -> String {
    TEMPLATE
        .replace("{{CSS}}", CSS)
        .replace("UI_SCRIPT", SCRIPT)
}

#[cfg(test)]
mod tests {
    use super::page;

    #[test]
    fn embeds_the_client_ui() {
        let page = page();

        assert!(page.contains(r#"id="directories""#));
        assert!(page.contains(r#"id="action-menu""#));
        assert!(page.contains(r#"id="selection-menu""#));
        assert!(page.contains(r#"id="viewer-menu""#));
        assert_eq!(page.matches(r#"id="menu""#).count(), 1);
        assert_eq!(page.matches(r#"class="download""#).count(), 1);
        assert_eq!(page.matches(r#"class="to cp-to""#).count(), 1);
        assert!(!page.contains(r#"class="actions""#));
        assert!(page.contains(r#"data-zoom="reset""#));
        assert!(page.contains("IntersectionObserver"));
        assert!(page.contains(r#"params.set("metadata", "true")"#));
        assert!(!page.contains("{{CSS}}"));
        assert!(!page.contains("UI_SCRIPT"));
    }
}
