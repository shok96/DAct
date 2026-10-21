use std::path::{Path, PathBuf};

use crate::category::Category;
use crate::error::{DactError, Result};
use crate::ffmpeg::Ffmpeg;
use crate::paths::{sibling, unique_path};
use crate::settings::{AudioQuality, Settings};

const AUDIO_FORMATS: &[(&str, &str, &str, &str)] = &[
    ("MP3", "mp3", "libmp3lame", "mp3"),
    ("WAV", "wav", "pcm_s16le", "wav"),
    ("FLAC", "flac", "flac", "flac"),
    ("M4A", "m4a", "aac", "ipod"),
    ("AAC", "aac", "aac", "adts"),
    ("AIFF", "aiff", "pcm_s16be", "aiff"),
    ("OGG", "ogg", "libvorbis", "ogg"),
    ("OPUS", "opus", "libopus", "opus"),
];

#[derive(Clone, Copy)]
struct VideoFmt {
    name: &'static str,
    ext: &'static str,
    muxer: &'static str,
    codec_variant: bool,
}

const VIDEO_CORE: &[VideoFmt] = &[
    VideoFmt { name: "MP4", ext: "mp4", muxer: "mp4", codec_variant: false },
    VideoFmt { name: "MOV", ext: "mov", muxer: "mov", codec_variant: false },
    VideoFmt { name: "MKV", ext: "mkv", muxer: "matroska", codec_variant: false },
    VideoFmt { name: "WEBM", ext: "webm", muxer: "webm", codec_variant: false },
    VideoFmt { name: "AVI", ext: "avi", muxer: "avi", codec_variant: false },
    VideoFmt { name: "M4V", ext: "m4v", muxer: "ipod", codec_variant: false },
];

#[cfg(target_os = "macos")]
const VIDEO_MAC: &[VideoFmt] = &[
    VideoFmt { name: "HEVC", ext: "mp4", muxer: "mp4", codec_variant: true },
    VideoFmt { name: "ProRes", ext: "mov", muxer: "mov", codec_variant: true },
    VideoFmt { name: "GIF", ext: "gif", muxer: "gif", codec_variant: false },
    VideoFmt { name: "MPEG", ext: "mpg", muxer: "mpeg", codec_variant: false },
    VideoFmt { name: "TS", ext: "ts", muxer: "mpegts", codec_variant: false },
];

fn video_formats() -> Vec<VideoFmt> {
    #[cfg(target_os = "macos")]
    {
        let mut out = VIDEO_CORE.to_vec();
        out.extend_from_slice(VIDEO_MAC);
        out
    }
    #[cfg(not(target_os = "macos"))]
    {
        VIDEO_CORE.to_vec()
    }
}

pub fn audio_targets(src: &Path) -> Vec<String> {
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    AUDIO_FORMATS
        .iter()
        .filter(|(_, e, _, _)| *e != ext)
        .map(|(n, _, _, _)| (*n).to_string())
        .collect()
}

pub fn video_targets(src: &Path) -> Vec<String> {
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    video_formats()
        .into_iter()
        .filter(|f| f.codec_variant || f.ext != ext)
        .map(|f| f.name.to_string())
        .collect()
}

pub fn convert_media(
    src: &Path,
    target: &str,
    kind: Category,
    extract_audio: bool,
    settings: &Settings,
    on_progress: &mut dyn FnMut(f32),
) -> Result<PathBuf> {
    let ff = Ffmpeg::resolve(settings)?;
    let target = target.to_ascii_uppercase();
    let (ext, args_tail) = if kind == Category::Audio || extract_audio {
        audio_args(&target, settings)?
    } else {
        video_args(&target)?
    };
    let label = match target.as_str() {
        "HEVC" => crate::i18n::suffix_hevc(),
        "ProRes" => crate::i18n::suffix_prores(),
        _ => "",
    };
    let out = sibling(src, label, &ext, settings.output)?;
    let tmp = unique_path(out.with_file_name(format!(
        ".~{}.part.{}",
        out.file_stem().and_then(|s| s.to_str()).unwrap_or("out"),
        ext
    )));

    let extra: Vec<&str> = {
        let mut v = vec!["-map_metadata", "0"];
        v.extend(args_tail.iter().map(String::as_str));
        v
    };
    let result = ff.convert_file_with_progress(src, &tmp, &extra, on_progress);
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
        return result.map(|_| out);
    }
    std::fs::rename(&tmp, &out).map_err(DactError::from_io)?;
    Ok(out)
}

fn audio_args(target: &str, settings: &Settings) -> Result<(String, Vec<String>)> {
    let (name, ext, encoder, muxer) = AUDIO_FORMATS
        .iter()
        .find(|(n, _, _, _)| *n == target)
        .ok_or_else(|| DactError::user(crate::i18n::err_unknown_audio(target)))?;
    let _ = name;
    let mut args = vec![
        "-vn".into(),
        "-map".into(),
        "0:a:0".into(),
        "-c:a".into(),
        (*encoder).into(),
    ];
    if matches!(*encoder, "libmp3lame" | "aac" | "libvorbis" | "libopus") {
        args.push("-b:a".into());
        args.push(format!("{}k", bitrate(target, settings.audio_quality)));
    }
    if target == "WAV" {
        let codec = match settings.audio_wav_bit_depth.as_str() {
            "24" => "pcm_s24le",
            "32f" => "pcm_f32le",
            _ => "pcm_s16le",
        };
        if let Some(pos) = args.iter().position(|a| a == *encoder) {
            args[pos] = codec.into();
        }
    }
    match settings.audio_channels.as_str() {
        "mono" => {
            args.push("-ac".into());
            args.push("1".into());
        }
        "stereo" => {
            args.push("-ac".into());
            args.push("2".into());
        }
        _ => {}
    }
    if let Some(rate) = settings.sample_rate() {
        args.push("-ar".into());
        args.push(rate.to_string());
    }
    args.push("-f".into());
    args.push((*muxer).into());
    Ok(((*ext).into(), args))
}

fn video_args(target: &str) -> Result<(String, Vec<String>)> {
    let fmt = video_formats()
        .into_iter()
        .find(|f| f.name == target)
        .ok_or_else(|| DactError::user(crate::i18n::err_unknown_video(target)))?;
    let mut args = match target {
        "WEBM" => vec![
            "-map",
            "0:v:0",
            "-map",
            "0:a?",
            "-c:v",
            "libvpx-vp9",
            "-deadline",
            "good",
            "-cpu-used",
            "2",
            "-crf",
            "31",
            "-b:v",
            "0",
            "-c:a",
            "libopus",
        ],
        "HEVC" => vec![
            "-map",
            "0:v:0",
            "-map",
            "0:a?",
            "-c:v",
            "hevc_videotoolbox",
            "-q:v",
            "45",
            "-tag:v",
            "hvc1",
            "-c:a",
            "aac",
        ],
        "ProRes" => vec![
            "-map",
            "0:v:0",
            "-map",
            "0:a?",
            "-c:v",
            "prores_ks",
            "-profile:v",
            "2",
            "-c:a",
            "pcm_s16le",
        ],
        "GIF" => vec![
            "-map",
            "0:v:0",
            "-an",
            "-vf",
            "fps=12,scale=min(720\\,iw):-2:flags=lanczos",
            "-c:v",
            "gif",
        ],
        "MPEG" => vec![
            "-map",
            "0:v:0",
            "-map",
            "0:a?",
            "-c:v",
            "mpeg2video",
            "-q:v",
            "5",
            "-c:a",
            "mp2",
            "-b:a",
            "192k",
        ],
        other => {
            let acodec = match other {
                "AVI" => "libmp3lame",
                "MKV" => "copy",
                _ => "aac",
            };
            vec![
                "-map",
                "0:v:0",
                "-map",
                "0:a?",
                "-c:v",
                "libx264",
                "-preset",
                "medium",
                "-crf",
                "21",
                "-c:a",
                acodec,
            ]
        }
    };
    args.extend(["-f", fmt.muxer]);
    Ok((
        fmt.ext.into(),
        args.into_iter().map(str::to_string).collect(),
    ))
}

fn bitrate(target: &str, quality: AudioQuality) -> u32 {
    match (quality, target) {
        (AudioQuality::Low, "OPUS") => 96,
        (AudioQuality::Low, _) => 128,
        (AudioQuality::Medium, "OPUS") => 128,
        (AudioQuality::Medium, _) => 192,
        (AudioQuality::High, "MP3") => 320,
        (AudioQuality::High, "OPUS") => 192,
        (AudioQuality::High, _) => 256,
    }
}

pub fn ffmpeg_available(settings: &Settings) -> bool {
    Ffmpeg::resolve(settings).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn video_targets_skip_same_container() {
        let t = video_targets(Path::new("clip.mp4"));
        assert!(t.contains(&"MOV".into()));
        assert!(!t.contains(&"MP4".into()));
        #[cfg(target_os = "macos")]
        {
            assert!(t.contains(&"HEVC".into()));
            assert!(t.contains(&"ProRes".into()));
            assert!(t.contains(&"GIF".into()));
        }
    }
}
