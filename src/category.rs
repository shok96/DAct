use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Image,
    Pdf,
    Docx,
    Text,
    Audio,
    Video,
}

pub const IMAGE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "tif", "tiff", "bmp", "gif", "ico", "heic", "heif", "avif",
];
pub const PDF_EXTS: &[&str] = &["pdf"];
pub const DOCX_EXTS: &[&str] = &["docx"];
pub const TEXT_EXTS: &[&str] = &["txt", "md", "csv", "log", "json", "xml", "html", "htm"];
pub const AUDIO_EXTS: &[&str] = &[
    "mp3", "wav", "flac", "m4a", "aac", "aiff", "aif", "ogg", "opus", "wma",
];
pub const VIDEO_EXTS: &[&str] = &[
    "mp4", "mov", "mkv", "webm", "avi", "m4v", "wmv", "mpg", "mpeg", "flv", "ts", "mts", "m2ts",
    "3gp", "asf", "vob", "qt", "dv", "hevc", "mxf", "3g2", "f4v", "ogv", "m2v", "m2t", "divx",
    "mod", "tod",
];

/// `Some(cat)` when every path is the same convertible type.
pub fn uniform(paths: &[impl AsRef<Path>]) -> Option<Category> {
    let mut cats = paths.iter().map(category);
    let first = cats.next()??;
    if cats.all(|c| c == Some(first)) {
        Some(first)
    } else {
        None
    }
}

pub fn category(path: impl AsRef<Path>) -> Option<Category> {
    let ext = path
        .as_ref()
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())?;
    let ext = ext.as_str();
    if AUDIO_EXTS.contains(&ext) {
        Some(Category::Audio)
    } else if VIDEO_EXTS.contains(&ext) {
        Some(Category::Video)
    } else if IMAGE_EXTS.contains(&ext) {
        Some(Category::Image)
    } else if PDF_EXTS.contains(&ext) {
        Some(Category::Pdf)
    } else if DOCX_EXTS.contains(&ext) {
        Some(Category::Docx)
    } else if TEXT_EXTS.contains(&ext) {
        Some(Category::Text)
    } else {
        None
    }
}

pub fn normalize_label(ext: &str) -> String {
    match ext.to_ascii_lowercase().as_str() {
        "jpeg" | "jpg" => "JPG".into(),
        "tif" | "tiff" => "TIFF".into(),
        "heif" | "heic" => "HEIC".into(),
        other => other.to_ascii_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_files() {
        assert_eq!(category("a.PNG"), Some(Category::Image));
        assert_eq!(category("a.jpeg"), Some(Category::Image));
        assert_eq!(category("notes.md"), Some(Category::Text));
        assert_eq!(category("song.flac"), Some(Category::Audio));
        assert_eq!(category("clip.mkv"), Some(Category::Video));
        assert_eq!(category("clip.hevc"), Some(Category::Video));
        assert_eq!(category("tape.dv"), Some(Category::Video));
        assert_eq!(category("doc.pdf"), Some(Category::Pdf));
        assert_eq!(category("x.exe"), None);
        assert_eq!(uniform(&["a.jpg", "b.png"]), Some(Category::Image));
        assert_eq!(uniform(&["a.jpg", "b.mp4"]), None);
        assert_eq!(uniform(&["clip.mkv", "other.mov"]), Some(Category::Video));
    }
}
