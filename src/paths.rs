use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{DactError, Result};
use crate::settings::OutputMode;

pub fn unique_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if !path.exists() {
        return path.to_path_buf();
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("файл");
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{s}"))
        .unwrap_or_default();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut i = 1u32;
    loop {
        let cand = parent.join(format!("{stem} ({i}){ext}"));
        if !cand.exists() {
            return cand;
        }
        i += 1;
    }
}

pub fn output_dir(src: impl AsRef<Path>, mode: OutputMode) -> Result<PathBuf> {
    let parent = src
        .as_ref()
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    match mode {
        OutputMode::NextToOriginal => Ok(parent),
        OutputMode::Subfolder => {
            let dir = parent.join("DAct");
            fs::create_dir_all(&dir).map_err(DactError::from_io)?;
            Ok(dir)
        }
    }
}

/// Output path next to `src`. Never overwrites an existing file.
pub fn sibling(
    src: impl AsRef<Path>,
    label: &str,
    ext: &str,
    mode: OutputMode,
) -> Result<PathBuf> {
    let src = src.as_ref();
    let dir = output_dir(src, mode)?;
    let stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("файл");
    let stem = if label.is_empty() {
        stem.to_string()
    } else {
        format!("{stem} {label}")
    };
    let ext = ext.trim_start_matches('.');
    Ok(unique_path(dir.join(format!("{stem}.{ext}"))))
}

pub fn human_size(n: u64) -> String {
    if n < 1024 {
        return format!("{n} Б");
    }
    let units = ["КБ", "МБ", "ГБ"];
    let mut f = n as f64;
    for (i, unit) in units.iter().enumerate() {
        f /= 1024.0;
        if f < 1024.0 || i == units.len() - 1 {
            return format!("{f:.1} {unit}");
        }
    }
    format!("{f:.1} ГБ")
}

pub fn file_size(path: impl AsRef<Path>) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

pub fn atomic_write(path: impl AsRef<Path>, bytes: &[u8]) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(DactError::from_io)?;
    }
    let tmp = path.with_file_name(format!(
        ".~{}.part",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("out")
    ));
    let result = (|| {
        fs::write(&tmp, bytes).map_err(DactError::from_io)?;
        fs::rename(&tmp, path).map_err(DactError::from_io)?;
        Ok(())
    })();
    if tmp.exists() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("DAct")
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("DAct")
}

pub fn log_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Logs/DAct/log.txt")
    }
    #[cfg(not(target_os = "macos"))]
    {
        data_dir().join("log.txt")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_avoids_existing() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("shot.png");
        fs::write(&a, b"x").unwrap();
        let b = unique_path(&a);
        assert_eq!(b.file_name().unwrap(), "shot (1).png");
        fs::write(&b, b"y").unwrap();
        let c = unique_path(&a);
        assert_eq!(c.file_name().unwrap(), "shot (2).png");
    }

    #[test]
    fn sizes_format() {
        assert_eq!(human_size(800), "800 Б");
        assert_eq!(human_size(1536), "1.5 КБ");
    }
}
