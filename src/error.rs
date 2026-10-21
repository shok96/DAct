use std::path::PathBuf;

use thiserror::Error;

/// Errors that are safe to show in the toast UI as-is.
#[derive(Debug, Error)]
pub enum DactError {
    #[error("{0}")]
    User(String),
    #[error("Файл перенесли или удалили: {0}")]
    Missing(PathBuf),
    #[error("Нет прав записать рядом с этим файлом. Выберите свою папку или в Настройках включите сохранение в подпапку DAct.")]
    Permission,
    #[error("DAct не читает этот формат.")]
    Unsupported,
    #[error("FFmpeg не найден. Нажмите «Скачать FFmpeg» или укажите путь в Настройках.")]
    NoFfmpeg,
    #[error("FFmpeg не смог перекодировать файл в выбранный формат.")]
    FfmpegFailed,
    #[error("В этом файле нет звуковой дорожки.")]
    NoAudio,
    #[error("PDF защищён паролем. Сначала снимите защиту.")]
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
            std::io::ErrorKind::NotFound => Self::User(crate::i18n::err_file_gone().into()),
            _ => Self::User(err.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, DactError>;
