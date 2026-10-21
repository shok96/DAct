# Changelog

## 0.1.0

First public source release. Tray app, radial menu, conversions, and a local FFmpeg install that only starts when you ask for it.

### How it works

- Hold Alt (or Shift / Ctrl) and the left mouse button. The dial opens under the cursor. Release on a slice.
- The source file is not overwritten. The result is written next to it, or into a `DAct` subdirectory. A taken name gets ` (1)`, ` (2)`, …
- One job is one category. A JPEG and an MP4 in the same drop are rejected.
- Closing the window leaves the process in the tray. Quit from the tray icon.
- A second launch forwards its file list to the running process.

On Windows the dial also reads the files Explorer has selected or is dragging. On macOS and Linux that path is not wired; drop files on the window, use Open files, or pass paths on the command line.

### Formats

| File | Convert to | Tools |
|---|---|---|
| Images (PNG, JPG, WEBP, TIFF, BMP, GIF, ICO; HEIC / HEIF / AVIF with FFmpeg) | Any of those, or PDF. Several images become one PDF | Compress · Resize · Rotate 90° · Metadata |
| PDF | TXT | Compress · Split · Rotate · Merge · Extract images · Metadata |
| Text (TXT, MD, CSV, LOG, JSON, XML, HTML) | PDF, DOCX | — |
| Word (DOCX) | TXT | — |
| Audio | MP3, WAV, FLAC, M4A, AAC, AIFF, OGG, OPUS | — |
| Video | MP4, MOV, MKV, WEBM, AVI, M4V | Extract audio to MP3, AAC, or FLAC |

Images, PDF, and text convert in-process. Audio, video, HEIC, HEIF, and AVIF go through a local `ffmpeg`. Video is re-encoded: WebM is VP9, the other containers are H.264. DOCX → TXT keeps paragraph text and drops layout. An encrypted PDF is not opened.

### FFmpeg

**Get FFmpeg** writes `ffmpeg` and `ffprobe` into the user data folder. Nothing is downloaded at launch. A binary already on `PATH`, or a path set in Settings, is used instead.

| System | Source |
|---|---|
| Windows | [gyan.dev essentials](https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip) |
| macOS x86_64 / arm64 | [FFmpegBin 9.0.2](https://github.com/Tyrrrz/FFmpegBin/releases/tag/9.0.2) |
| Linux x86_64 / arm64 | [BtbN FFmpeg 9.0 GPL](https://github.com/BtbN/FFmpeg-Builds/releases/tag/latest) |

On macOS the downloaded file is taken out of quarantine and ad-hoc signed so Gatekeeper will run it. Other CPU architectures are not fetched; install `ffmpeg` yourself and set the path.

These archives are not in the repository. The MIT license on DAct does not cover them.

### Windows overlays

The dial and the notice cards are clipped to the painted pixels. DWM glass / acrylic behind the HWND is off. When the overlay is idle it is hidden, not left as an empty topmost window.

### Command line

```
dact
dact --quiet
dact path/to/file.png
dact convert path/to/file.png --to WEBP
dact --selftest
dact --version
```

`--selftest` covers image and text conversions. It does not download or run FFmpeg.

### Paths

| | Settings | Log | Downloaded FFmpeg |
|---|---|---|---|
| Windows | `%APPDATA%\DAct\settings.toml` | `%LOCALAPPDATA%\DAct\log.txt` | `%LOCALAPPDATA%\DAct\ffmpeg` |
| macOS | `~/Library/Application Support/DAct/settings.toml` | `~/Library/Logs/DAct/log.txt` | `~/Library/Application Support/DAct/ffmpeg` |
| Linux | `~/.config/DAct/settings.toml` | `~/.local/share/DAct/log.txt` | `~/.local/share/DAct/ffmpeg` |

Start at login writes `dact --quiet` into the usual per-user slot for that OS (Run key, LaunchAgent, or `autostart/dact.desktop`).
