pub mod delete;
pub mod download;
pub mod download_bulk;
pub mod file_server;
pub mod list;
pub mod mkdir;
pub mod mv;
pub mod save;
pub mod search;
pub mod ui;
pub mod upload;

// Re-export handler functions
pub use delete::handle_delete;
pub use download::handle_download;
pub use download_bulk::handle_downloads;
pub use file_server::handle_file_server;
pub use list::handle_list;
pub use mkdir::handle_mkdir;
pub use mv::handle_mv;
pub use save::handle_save;
pub use search::handle_search;
pub use ui::ui_routes;
pub use upload::handle_upload;
