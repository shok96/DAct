use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::error::{DactError, Result};
use crate::paths::data_dir;
use crate::settings::Settings;

#[cfg(windows)]
const FFMPEG_ZIP: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

/// Pinned release. The archive is a flat zip of `ffmpeg` and `ffprobe`
/// (plus `ffplay`, which is ignored). See https://github.com/Tyrrrz/FFmpegBin
#[cfg(target_os = "macos")]
const FFMPEG_BIN_VERSION: &str = "9.0.2";

/// Local FFmpeg only. Download happens only when the user asks.
#[derive(Debug, Clone)]
pub struct Ffmpeg {
    pub exe: PathBuf,
}

impl Ffmpeg {
    pub fn resolve(settings: &Settings) -> Result<Self> {
        for path in candidates(settings) {
            if path.is_file() {
                return Ok(Self { exe: path });
            }
        }
        Err(DactError::NoFfmpeg)
    }

    pub fn convert_file(&self, src: &Path, dst: &Path, extra: &[&str]) -> Result<()> {
        self.convert_file_with_progress(src, dst, extra, &mut |_| {})
    }

    pub fn convert_file_with_progress(
        &self,
        src: &Path,
        dst: &Path,
        extra: &[&str],
        on_progress: &mut dyn FnMut(f32),
    ) -> Result<()> {
        let known = probe_duration(&self.exe, src);
        let mut cmd = Command::new(&self.exe);
        cmd.arg("-hide_banner")
            .arg("-nostdin")
            .arg("-y")
            .arg("-i")
            .arg(src)
            .args(extra)
            .arg("-progress")
            .arg("pipe:1")
            .arg("-nostats")
            .arg(dst)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        silence(&mut cmd);
        let mut child = cmd.spawn().map_err(|_| DactError::NoFfmpeg)?;
        let stdout = child.stdout.take().ok_or(DactError::NoFfmpeg)?;
        let stderr = child.stderr.take().ok_or(DactError::NoFfmpeg)?;
        let duration = Arc::new(Mutex::new(known));
        let err_buf = Arc::new(Mutex::new(String::new()));
        let duration_w = Arc::clone(&duration);
        let err_w = Arc::clone(&err_buf);
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(|l| l.ok()) {
                if let Some(d) = parse_duration_line(&line) {
                    if let Ok(mut g) = duration_w.lock() {
                        *g = Some(d);
                    }
                }
                if let Ok(mut g) = err_w.lock() {
                    g.push_str(&line);
                    g.push('\n');
                }
            }
        });
        for line in BufReader::new(stdout).lines().map_while(|l| l.ok()) {
            if let Some(t) = parse_out_time(&line) {
                let dur = duration.lock().ok().and_then(|g| *g).unwrap_or(0.0);
                let frac = if dur > 0.05 {
                    (t / dur).clamp(0.0, 0.99)
                } else {
                    0.0
                };
                on_progress(frac);
            }
        }
        let status = child.wait().map_err(|_| DactError::FfmpegFailed)?;
        let err = err_buf.lock().map(|g| g.clone()).unwrap_or_default();
        if !status.success() {
            let low = err.to_ascii_lowercase();
            if low.contains("matches no streams")
                || low.contains("does no contain any stream")
                || low.contains("does not contain any stream")
            {
                return Err(DactError::NoAudio);
            }
            if low.contains("no space left") {
                return Err(DactError::user(crate::i18n::err_disk_full()));
            }
            return Err(DactError::FfmpegFailed);
        }
        if !dst.is_file() || dst.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
            return Err(DactError::FfmpegFailed);
        }
        on_progress(1.0);
        Ok(())
    }
}

fn probe_duration(ffmpeg: &Path, src: &Path) -> Option<f32> {
    let named = if cfg!(windows) {
        "ffprobe.exe"
    } else {
        "ffprobe"
    };
    let probe = ffmpeg.parent()?.join(named);
    if !probe.is_file() {
        return None;
    }
    let mut cmd = Command::new(probe);
    cmd.args([
        "-v",
        "error",
        "-show_entries",
        "format=duration",
        "-of",
        "default=noprint_wrappers=1:nokey=1",
    ])
    .arg(src)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    silence(&mut cmd);
    let out = cmd.output().ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

fn parse_duration_line(line: &str) -> Option<f32> {
    let rest = line.split("Duration:").nth(1)?;
    let token = rest.split(',').next()?.trim();
    parse_hms(token)
}

fn parse_out_time(line: &str) -> Option<f32> {
    if let Some(v) = line.strip_prefix("out_time_us=") {
        return v.trim().parse::<f64>().ok().map(|u| (u / 1_000_000.0) as f32);
    }
    if let Some(v) = line.strip_prefix("out_time_ms=") {
        return v.trim().parse::<f64>().ok().map(|u| (u / 1_000_000.0) as f32);
    }
    line.strip_prefix("out_time=")
        .and_then(|v| parse_hms(v.trim()))
}

fn parse_hms(s: &str) -> Option<f32> {
    let mut parts = s.split(':');
    let h: f32 = parts.next()?.parse().ok()?;
    let m: f32 = parts.next()?.parse().ok()?;
    let sec: f32 = parts.next()?.parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + sec)
}

pub fn silence(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd;
}

pub fn bundled_dir() -> PathBuf {
    data_dir().join("ffmpeg")
}

pub fn bundled_exe() -> PathBuf {
    let named = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    bundled_dir().join(named)
}

/// User-started install into the per-user DAct data folder. Never runs on launch.
pub fn install() -> Result<PathBuf> {
    let dest = bundled_exe();
    if dest.is_file() {
        return Ok(dest);
    }
    let dir = bundled_dir();
    fs::create_dir_all(&dir).map_err(DactError::from_io)?;
    let url = ffmpeg_archive_url()?;
    let archive_name = if url.ends_with(".tar.xz") {
        "ffmpeg-download.tar.xz"
    } else {
        "ffmpeg-download.zip"
    };
    let archive_path = data_dir().join(archive_name);
    let fetched = download(&url, &archive_path);
    if let Err(err) = fetched {
        let _ = fs::remove_file(&archive_path);
        return Err(err);
    }
    let extracted = if url.ends_with(".tar.xz") {
        extract_tar_xz(&archive_path, &dir)
    } else {
        extract_tools(&archive_path, &dir)
    };
    let _ = fs::remove_file(&archive_path);
    extracted?;
    if !dest.is_file() {
        return Err(DactError::user(
            crate::i18n::err_ffmpeg_missing_bin(),
        ));
    }
    mark_tool(&dest)?;
    let probe = dest.with_file_name(probe_name());
    if probe.is_file() {
        mark_tool(&probe)?;
    }
    Ok(dest)
}

fn ffmpeg_archive_url() -> Result<String> {
    #[cfg(windows)]
    {
        Ok(FFMPEG_ZIP.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        let arch = match std::env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            other => {
                return Err(DactError::user(crate::i18n::err_ffmpeg_no_mac(other)));
            }
        };
        Ok(format!(
            "https://github.com/Tyrrrz/FFmpegBin/releases/download/{FFMPEG_BIN_VERSION}/ffmpeg-osx-{arch}.zip"
        ))
    }
    #[cfg(target_os = "linux")]
    {
        // GPL static builds. The rolling `latest` tag keeps this file name.
        // Linked against an older glibc than a binary built on current Ubuntu.
        let name = match std::env::consts::ARCH {
            "x86_64" => "ffmpeg-n9.0-latest-linux64-gpl-9.0.tar.xz",
            "aarch64" => "ffmpeg-n9.0-latest-linuxarm64-gpl-9.0.tar.xz",
            other => {
                return Err(DactError::user(crate::i18n::err_ffmpeg_no_linux(other)));
            }
        };
        Ok(format!(
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/{name}"
        ))
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        let _ = ();
        Err(DactError::user(
            crate::i18n::err_ffmpeg_no_os(),
        ))
    }
}

fn probe_name() -> &'static str {
    if cfg!(windows) {
        "ffprobe.exe"
    } else {
        "ffprobe"
    }
}

fn download(url: &str, dest: &Path) -> Result<()> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(600))
        .redirects(8)
        .build();
    let resp = agent
        .get(url)
        .set("User-Agent", "DAct")
        .call()
        .map_err(|e| DactError::user(crate::i18n::err_ffmpeg_download(e)))?;
    let mut file = File::create(dest).map_err(DactError::from_io)?;
    let mut reader = resp.into_reader();
    std::io::copy(&mut reader, &mut file).map_err(DactError::from_io)?;
    Ok(())
}

fn tool_file_name(entry_name: &str) -> Option<String> {
    let name = entry_name.replace('\\', "/");
    let file_name = name.rsplit('/').next().unwrap_or("");
    match file_name {
        "ffmpeg" | "ffprobe" | "ffmpeg.exe" | "ffprobe.exe" => Some(file_name.to_string()),
        _ => None,
    }
}

fn extract_tools(zip_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = File::open(zip_path).map_err(DactError::from_io)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| DactError::user(crate::i18n::err_ffmpeg_archive(e)))?;
    let mut staged: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut found = false;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| DactError::user(crate::i18n::err_ffmpeg_archive_read(e)))?;
        if !entry.is_file() {
            continue;
        }
        let Some(file_name) = tool_file_name(entry.name()) else {
            continue;
        };
        let part = dest_dir.join(format!(".{file_name}.part"));
        let final_path = dest_dir.join(&file_name);
        let copied = (|| {
            let mut out = File::create(&part).map_err(DactError::from_io)?;
            std::io::copy(&mut entry, &mut out).map_err(DactError::from_io)?;
            Ok(())
        })();
        if let Err(err) = copied {
            let _ = fs::remove_file(&part);
            for (part, _) in &staged {
                let _ = fs::remove_file(part);
            }
            return Err(err);
        }
        if file_name == "ffmpeg" || file_name == "ffmpeg.exe" {
            found = true;
        }
        staged.push((part, final_path));
    }
    if !found {
        for (part, _) in &staged {
            let _ = fs::remove_file(part);
        }
        return Err(DactError::user(
            crate::i18n::err_ffmpeg_no_bin(),
        ));
    }
    for (part, final_path) in staged {
        fs::rename(&part, &final_path).map_err(DactError::from_io)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn extract_tar_xz(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = File::open(archive_path).map_err(DactError::from_io)?;
    let decoder = xz2::read::XzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|e| DactError::user(crate::i18n::err_ffmpeg_archive_read(e)))?;
    let mut staged: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut found = false;
    for entry in entries {
        let mut entry =
            entry.map_err(|e| DactError::user(crate::i18n::err_ffmpeg_archive_read(e)))?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let file_name = {
            let path = entry
                .path()
                .map_err(|e| DactError::user(crate::i18n::err_ffmpeg_archive_read(e)))?;
            path.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        };
        let Some(file_name) = file_name else {
            continue;
        };
        if file_name != "ffmpeg" && file_name != "ffprobe" {
            continue;
        }
        let part = dest_dir.join(format!(".{file_name}.part"));
        let final_path = dest_dir.join(&file_name);
        let copied = (|| {
            let mut out = File::create(&part).map_err(DactError::from_io)?;
            std::io::copy(&mut entry, &mut out).map_err(DactError::from_io)?;
            Ok(())
        })();
        if let Err(err) = copied {
            let _ = fs::remove_file(&part);
            for (part, _) in &staged {
                let _ = fs::remove_file(part);
            }
            return Err(err);
        }
        if file_name == "ffmpeg" {
            found = true;
        }
        staged.push((part, final_path));
    }
    if !found {
        for (part, _) in &staged {
            let _ = fs::remove_file(part);
        }
        return Err(DactError::user(
            crate::i18n::err_ffmpeg_no_bin(),
        ));
    }
    for (part, final_path) in staged {
        fs::rename(&part, &final_path).map_err(DactError::from_io)?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn extract_tar_xz(_archive_path: &Path, _dest_dir: &Path) -> Result<()> {
    Err(DactError::user(
        crate::i18n::err_ffmpeg_linux_only(),
    ))
}

fn mark_tool(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path).map_err(DactError::from_io)?.permissions();
        perms.set_mode(perms.mode() | 0o755);
        fs::set_permissions(path, perms).map_err(DactError::from_io)?;
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("/usr/bin/xattr")
                .args(["-d", "com.apple.quarantine"])
                .arg(path)
                .status();
            let _ = Command::new("/usr/bin/codesign")
                .args(["--force", "--sign", "-"])
                .arg(path)
                .status();
        }
    }
    let _ = path;
    Ok(())
}

fn candidates(settings: &Settings) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let named = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    if !settings.ffmpeg_path.trim().is_empty() {
        out.push(PathBuf::from(settings.ffmpeg_path.trim()));
    }
    out.push(bundled_exe());
    out.push(bundled_dir().join("bin").join(named));
    if let Ok(p) = which::which(named) {
        out.push(p);
    }
    if let Some(home) = dirs::home_dir() {
        out.push(home.join(format!("bin/{named}")));
        out.push(home.join(format!("ffmpeg/bin/{named}")));
    }
    if let Some(local) = dirs::data_local_dir() {
        out.push(local.join(format!("Microsoft/WinGet/Links/{named}")));
        out.push(local.join(format!("Programs/ffmpeg/{named}")));
        out.push(local.join(format!("ffmpeg/bin/{named}")));
    }
    #[cfg(windows)]
    {
        out.push(PathBuf::from(r"C:\ffmpeg\bin\ffmpeg.exe"));
        if let Some(pf) = std::env::var_os("ProgramFiles") {
            out.push(PathBuf::from(pf).join(r"ffmpeg\bin\ffmpeg.exe"));
        }
        if let Some(pf86) = std::env::var_os("ProgramFiles(x86)") {
            out.push(PathBuf::from(pf86).join(r"ffmpeg\bin\ffmpeg.exe"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        out.push(PathBuf::from("/opt/homebrew/bin/ffmpeg"));
        out.push(PathBuf::from("/usr/local/bin/ffmpeg"));
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        out.push(PathBuf::from("/usr/bin/ffmpeg"));
        out.push(PathBuf::from("/usr/local/bin/ffmpeg"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_names_ignore_directories_and_extra_files() {
        assert_eq!(tool_file_name("bin/ffmpeg").as_deref(), Some("ffmpeg"));
        assert_eq!(tool_file_name("ffmpeg.exe").as_deref(), Some("ffmpeg.exe"));
        assert_eq!(
            tool_file_name(r"ffmpeg-essentials\bin\ffprobe.exe").as_deref(),
            Some("ffprobe.exe")
        );
        assert_eq!(tool_file_name("ffplay"), None);
        assert_eq!(tool_file_name("readme.txt"), None);
    }

    #[test]
    fn extract_keeps_only_ffmpeg_and_ffprobe() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("ffmpeg.zip");
        {
            let file = File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("build/bin/ffmpeg", opts).unwrap();
            std::io::Write::write_all(&mut writer, b"ffmpeg-bytes").unwrap();
            writer.start_file("build/bin/ffprobe", opts).unwrap();
            std::io::Write::write_all(&mut writer, b"ffprobe-bytes").unwrap();
            writer.start_file("build/bin/ffplay", opts).unwrap();
            std::io::Write::write_all(&mut writer, b"ffplay-bytes").unwrap();
            writer.finish().unwrap();
        }
        let out = dir.path().join("out");
        fs::create_dir_all(&out).unwrap();
        extract_tools(&zip_path, &out).unwrap();
        assert_eq!(fs::read(out.join("ffmpeg")).unwrap(), b"ffmpeg-bytes");
        assert_eq!(fs::read(out.join("ffprobe")).unwrap(), b"ffprobe-bytes");
        assert!(!out.join("ffplay").exists());
    }
}
