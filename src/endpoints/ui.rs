use warp::Filter;
use warp::Reply;

fn text_response(content: &'static str, content_type: &'static str) -> warp::reply::Response {
    warp::reply::with_header(content, "content-type", content_type).into_response()
}

pub fn ui_routes() -> warp::filters::BoxedFilter<(warp::reply::Response,)> {
    let ui_index = warp::path("ui").and(warp::path::end()).map(|| {
        warp::reply::html(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/static/index.html"
        )))
        .into_response()
    });

    let ui_css_base = warp::path("ui")
        .and(warp::path("css"))
        .and(warp::path("base.css"))
        .map(|| {
            text_response(
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/css/base.css")),
                "text/css",
            )
        });

    let ui_css_layout = warp::path("ui")
        .and(warp::path("css"))
        .and(warp::path("layout.css"))
        .map(|| {
            text_response(
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/css/layout.css")),
                "text/css",
            )
        });

    let ui_css_components = warp::path("ui")
        .and(warp::path("css"))
        .and(warp::path("components.css"))
        .map(|| {
            text_response(
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/static/css/components.css"
                )),
                "text/css",
            )
        });

    let ui_css_viewer = warp::path("ui")
        .and(warp::path("css"))
        .and(warp::path("viewer.css"))
        .map(|| {
            text_response(
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/css/viewer.css")),
                "text/css",
            )
        });

    let ui_css_responsive = warp::path("ui")
        .and(warp::path("css"))
        .and(warp::path("responsive.css"))
        .map(|| {
            text_response(
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/static/css/responsive.css"
                )),
                "text/css",
            )
        });

    let ui_js_state = warp::path("ui")
        .and(warp::path("js"))
        .and(warp::path("state.js"))
        .map(|| {
            text_response(
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/js/state.js")),
                "application/javascript",
            )
        });

    let ui_js_utils = warp::path("ui")
        .and(warp::path("js"))
        .and(warp::path("utils.js"))
        .map(|| {
            text_response(
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/js/utils.js")),
                "application/javascript",
            )
        });

    let ui_js_navigation = warp::path("ui")
        .and(warp::path("js"))
        .and(warp::path("navigation.js"))
        .map(|| {
            text_response(
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/js/navigation.js")),
                "application/javascript",
            )
        });

    let ui_js_gallery = warp::path("ui")
        .and(warp::path("js"))
        .and(warp::path("gallery.js"))
        .map(|| {
            text_response(
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/js/gallery.js")),
                "application/javascript",
            )
        });

    let ui_js_selection = warp::path("ui")
        .and(warp::path("js"))
        .and(warp::path("selection.js"))
        .map(|| {
            text_response(
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/js/selection.js")),
                "application/javascript",
            )
        });

    let ui_js_media_viewer = warp::path("ui")
        .and(warp::path("js"))
        .and(warp::path("media-viewer.js"))
        .map(|| {
            text_response(
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/static/js/media-viewer.js"
                )),
                "application/javascript",
            )
        });

    let ui_js_media_controls = warp::path("ui")
        .and(warp::path("js"))
        .and(warp::path("media-controls.js"))
        .map(|| {
            text_response(
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/static/js/media-controls.js"
                )),
                "application/javascript",
            )
        });

    let ui_js_file_ops = warp::path("ui")
        .and(warp::path("js"))
        .and(warp::path("file-ops.js"))
        .map(|| {
            text_response(
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/js/file-ops.js")),
                "application/javascript",
            )
        });

    let ui_js_main = warp::path("ui")
        .and(warp::path("js"))
        .and(warp::path("main.js"))
        .map(|| {
            text_response(
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/js/main.js")),
                "application/javascript",
            )
        });

    let ui_fallback = warp::path("ui").and(warp::path::tail()).map(|_tail: warp::path::Tail| {
        warp::reply::html(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/static/index.html"
        )))
        .into_response()
    });

    ui_index
        .or(ui_css_base)
        .unify()
        .or(ui_css_layout)
        .unify()
        .or(ui_css_components)
        .unify()
        .or(ui_css_viewer)
        .unify()
        .or(ui_css_responsive)
        .unify()
        .or(ui_js_state)
        .unify()
        .or(ui_js_utils)
        .unify()
        .or(ui_js_navigation)
        .unify()
        .or(ui_js_gallery)
        .unify()
        .or(ui_js_selection)
        .unify()
        .or(ui_js_media_viewer)
        .unify()
        .or(ui_js_media_controls)
        .unify()
        .or(ui_js_file_ops)
        .unify()
        .or(ui_js_main)
        .unify()
        .or(ui_fallback)
        .unify()
        .boxed()
}
