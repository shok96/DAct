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
                return Err(DactError::user("Not enough free disk space."));
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

/// User-started install into %LOCALAPPDATA%\DAct\ffmpeg. Never runs on launch.
pub fn install() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        install_windows()
    }
    #[cfg(not(windows))]
    {
        Err(DactError::user(
            "Install FFmpeg with your package manager, then set the path in Settings.",
        ))
    }
}

#[cfg(windows)]
fn install_windows() -> Result<PathBuf> {
    let dest = bundled_exe();
    if dest.is_file() {
        return Ok(dest);
    }
    let dir = bundled_dir();
    fs::create_dir_all(&dir).map_err(DactError::from_io)?;
    let zip_path = data_dir().join("ffmpeg-essentials.zip");
    download(FFMPEG_ZIP, &zip_path)?;
    extract_ffmpeg(&zip_path, &dir)?;
    let _ = fs::remove_file(&zip_path);
    if dest.is_file() {
        Ok(dest)
    } else {
        Err(DactError::user(
            "Downloaded FFmpeg, but ffmpeg.exe was not in the archive.",
        ))
    }
}

#[cfg(windows)]
fn download(url: &str, dest: &Path) -> Result<()> {
    let resp = ureq::get(url)
        .timeout(std::time::Duration::from_secs(180))
        .call()
        .map_err(|e| DactError::user(format!("Could not download FFmpeg: {e}")))?;
    let mut file = File::create(dest).map_err(DactError::from_io)?;
    let mut reader = resp.into_reader();
    std::io::copy(&mut reader, &mut file).map_err(DactError::from_io)?;
    Ok(())
}

#[cfg(windows)]
fn extract_ffmpeg(zip_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = File::open(zip_path).map_err(DactError::from_io)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| DactError::user(format!("Could not open FFmpeg archive: {e}")))?;
    let mut found = false;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| DactError::user(format!("Could not read FFmpeg archive: {e}")))?;
        if !entry.is_file() {
            continue;
        }
        let name = entry.name().replace('\\', "/");
        let file_name = name.rsplit('/').next().unwrap_or("");
        if file_name != "ffmpeg.exe" && file_name != "ffprobe.exe" {
            continue;
        }
        let out = dest_dir.join(file_name);
        let mut dest = File::create(&out).map_err(DactError::from_io)?;
        std::io::copy(&mut entry, &mut dest).map_err(DactError::from_io)?;
        if file_name == "ffmpeg.exe" {
            found = true;
        }
    }
    if found {
        Ok(())
    } else {
        Err(DactError::user(
            "The FFmpeg archive did not contain ffmpeg.exe.",
        ))
    }
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
