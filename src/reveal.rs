use std::path::Path;
use std::process::Command;

pub fn open_file(path: &Path) {
    let _ = open::that_detached(path);
}

pub fn reveal(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("explorer")
            .arg("/select,")
            .arg(path)
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg("-R").arg(path).spawn();
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(parent) = path.parent() {
            let _ = Command::new("xdg-open").arg(parent).spawn();
        }
    }
}

pub fn trash_paths(paths: &[std::path::PathBuf]) -> usize {
    let mut n = 0;
    for p in paths {
        if p.is_file() && trash::delete(p).is_ok() {
            n += 1;
        }
    }
    n
}
