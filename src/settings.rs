use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::paths::config_dir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TriggerKey {
    Shift,
    Ctrl,
    Alt,
}

impl TriggerKey {
    pub fn label(self) -> &'static str {
        match self {
            Self::Shift => {
                if cfg!(target_os = "macos") {
                    "Shift ⇧"
                } else {
                    "Shift"
                }
            }
            Self::Ctrl => {
                if cfg!(target_os = "macos") {
                    "Control ⌃"
                } else {
                    "Ctrl"
                }
            }
            Self::Alt => {
                if cfg!(target_os = "macos") {
                    "Option ⌥"
                } else {
                    "Alt"
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    #[serde(rename = "next")]
    NextToOriginal,
    #[serde(rename = "folder")]
    Subfolder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioQuality {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub trigger: TriggerKey,
    pub output: OutputMode,
    pub jpg_quality: u8,
    pub pdf_dpi: u16,
    pub audio_quality: AudioQuality,
    pub audio_sample_rate: String,
    pub audio_wav_bit_depth: String,
    pub audio_channels: String,
    pub ffmpeg_path: String,
    pub autostart: bool,
    pub welcomed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            trigger: TriggerKey::Alt,
            output: OutputMode::NextToOriginal,
            jpg_quality: 90,
            pdf_dpi: 200,
            audio_quality: AudioQuality::High,
            audio_sample_rate: "auto".into(),
            audio_wav_bit_depth: "auto".into(),
            audio_channels: "auto".into(),
            ffmpeg_path: String::new(),
            autostart: false,
            welcomed: false,
        }
    }
}

impl Settings {
    pub fn path() -> PathBuf {
        config_dir().join("settings.toml")
    }

    pub fn load() -> Self {
        let path = Self::path();
        let Ok(raw) = fs::read_to_string(&path) else {
            return Self::default();
        };
        toml::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(parent) = Self::path().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(Self::path(), toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn jpg_quality(&self) -> u8 {
        self.jpg_quality.clamp(30, 100)
    }

    pub fn sample_rate(&self) -> Option<u32> {
        if self.audio_sample_rate == "auto" {
            None
        } else {
            self.audio_sample_rate.parse().ok()
        }
    }
}
