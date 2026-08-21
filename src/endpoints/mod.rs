#[cfg(feature = "api")]
pub mod api;
pub mod httpd;
#[cfg(feature = "ui")]
pub mod ui;

pub use httpd::handle_file_server;
#[cfg(feature = "ui")]
pub(crate) use ui::handle_ui_path;
#[cfg(feature = "ui")]
pub use ui::{handle_ui, handle_ui_request};
