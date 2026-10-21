use std::path::PathBuf;

use thiserror::Error;

/// Errors that are safe to show in the toast UI as-is.
#[derive(Debug, Error)]
pub enum DactError {
    #[error("{0}")]
    User(String),
    #[error("The file was moved or deleted: {0}")]
    Missing(PathBuf),
    #[error("No permission to write next to this file. Try a folder you own, or set Settings → Save results → subfolder.")]
    Permission,
    #[error("This file isn't a format DAct can read.")]
    Unsupported,
    #[error("FFmpeg is not available. Press Get FFmpeg, or set its path in Settings.")]
    NoFfmpeg,
    #[error("FFmpeg could not convert this file with the selected format.")]
    FfmpegFailed,
    #[error("This file has no audio stream to convert.")]
    NoAudio,
    #[error("This PDF is password-protected. Remove the password first.")]
    PdfLocked,
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl DactError {
    pub fn user(msg: impl Into<String>) -> Self {
        Self::User(msg.into())
    }

    pub fn from_io(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::PermissionDenied => Self::Permission,
            std::io::ErrorKind::NotFound => Self::User("The file was moved or deleted.".into()),
            _ => Self::User(err.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, DactError>;
