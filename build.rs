use flate2::{Compression, GzBuilder};
use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;

const TEMPLATE_PATH: &str = "src/endpoints/ui/index.html";
const CSS_PATH: &str = "src/endpoints/ui/style.css";
const SCRIPT_PATH: &str = "src/endpoints/ui/script.js";
const CSS_MARKER: &str = "{{CSS}}";
const SCRIPT_MARKER: &str = "UI_SCRIPT";

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn contains_closing_tag(source: &str, tag: &[u8]) -> bool {
    source
        .as_bytes()
        .windows(tag.len())
        .any(|part| part.eq_ignore_ascii_case(tag))
}

fn assemble(template: &str, css: &str, script: &str) -> io::Result<String> {
    if template.matches(CSS_MARKER).count() != 1 {
        return Err(invalid("UI template must contain one CSS marker"));
    }
    if template.matches(SCRIPT_MARKER).count() != 1 {
        return Err(invalid("UI template must contain one script marker"));
    }
    if contains_closing_tag(css, b"</style") {
        return Err(invalid("UI CSS must not contain a closing style tag"));
    }
    if contains_closing_tag(script, b"</script") {
        return Err(invalid("UI script must not contain a closing script tag"));
    }

    let (head, rest) = template
        .split_once(CSS_MARKER)
        .ok_or_else(|| invalid("CSS marker must precede the script marker"))?;
    let (middle, tail) = rest
        .split_once(SCRIPT_MARKER)
        .ok_or_else(|| invalid("CSS marker must precede the script marker"))?;
    let size = template.len() - CSS_MARKER.len() - SCRIPT_MARKER.len() + css.len() + script.len();
    let mut page = String::with_capacity(size);
    page.push_str(head);
    page.push_str(css);
    page.push_str(middle);
    page.push_str(script);
    page.push_str(tail);
    Ok(page)
}

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed={TEMPLATE_PATH}");
    println!("cargo:rerun-if-changed={CSS_PATH}");
    println!("cargo:rerun-if-changed={SCRIPT_PATH}");

    let template = fs::read_to_string(TEMPLATE_PATH)?;
    let css = fs::read_to_string(CSS_PATH)?;
    let script = fs::read_to_string(SCRIPT_PATH)?;
    let page = assemble(&template, &css, &script)?;
    let output = PathBuf::from(
        env::var_os("OUT_DIR").ok_or_else(|| invalid("Cargo did not provide OUT_DIR"))?,
    );

    fs::write(output.join("ui.html"), page.as_bytes())?;
    let file = File::create(output.join("ui.html.gz"))?;
    let mut gzip = GzBuilder::new()
        .mtime(0)
        .operating_system(255)
        .write(file, Compression::best());
    gzip.write_all(page.as_bytes())?;
    gzip.finish()?;
    Ok(())
}
