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

const VIDEO_FORMATS: &[(&str, &str, &str)] = &[
    ("MP4", "mp4", "mp4"),
    ("MOV", "mov", "mov"),
    ("MKV", "mkv", "matroska"),
    ("WEBM", "webm", "webm"),
    ("AVI", "avi", "avi"),
    ("M4V", "m4v", "ipod"),
];

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
    VIDEO_FORMATS
        .iter()
        .filter(|(_, e, _)| *e != ext)
        .map(|(n, _, _)| (*n).to_string())
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
    let out = sibling(src, "", &ext, settings.output)?;
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
        .ok_or_else(|| DactError::user(format!("Unknown audio target {target}")))?;
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
    let (_, ext, muxer) = VIDEO_FORMATS
        .iter()
        .find(|(n, _, _)| *n == target)
        .ok_or_else(|| DactError::user(format!("Unknown video target {target}")))?;
    let (vcodec, extra) = if target == "WEBM" {
        (
            "libvpx-vp9",
            vec![
                "-deadline",
                "good",
                "-cpu-used",
                "2",
                "-crf",
                "31",
                "-b:v",
                "0",
            ],
        )
    } else {
        ("libx264", vec!["-preset", "medium", "-crf", "21"])
    };
    let acodec = match target {
        "WEBM" => "libopus",
        "AVI" => "libmp3lame",
        "MKV" => "copy",
        _ => "aac",
    };
    let mut args = vec![
        "-map".into(),
        "0:v:0".into(),
        "-map".into(),
        "0:a?".into(),
        "-c:v".into(),
        vcodec.into(),
    ];
    args.extend(extra.into_iter().map(|s| s.to_string()));
    args.push("-c:a".into());
    args.push(acodec.into());
    args.push("-f".into());
    args.push((*muxer).into());
    Ok(((*ext).into(), args))
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
