fn main() {
    println!("cargo::rustc-check-cfg=cfg(httpd)");
    println!("cargo::rustc-check-cfg=cfg(grid)");
    println!("cargo::rustc-check-cfg=cfg(ui)");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=HTTPD");
    println!("cargo:rerun-if-env-changed=GRID");
    println!("cargo:rerun-if-env-changed=UI");

    let is_set = |name: &str| std::env::var(name).as_deref() == Ok("1");

    let ui = is_set("UI");
    let grid = ui || is_set("GRID");
    let httpd = grid || is_set("HTTPD");

    if httpd {
        println!("cargo:rustc-cfg=httpd");
    }
    if grid {
        println!("cargo:rustc-cfg=grid");
    }
    if ui {
        println!("cargo:rustc-cfg=ui");
    }
}
