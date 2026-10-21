pub mod autostart;
pub mod category;
pub mod convert;
pub mod error;
pub mod ffmpeg;
pub mod i18n;
pub mod ipc;
pub mod notify;
pub mod paths;
pub mod report;
pub mod reveal;
pub mod settings;
pub mod ui;

pub const APP_NAME: &str = "DAct";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn setup_logging() {
    let path = paths::log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if path.exists() {
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() > 1_000_000 {
                let _ = std::fs::rename(&path, path.with_file_name("log.old.txt"));
            }
        }
    }
}
