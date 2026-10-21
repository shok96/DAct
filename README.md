# DAct

Convert files with a radial menu on Windows, macOS and Linux. Hold Alt, press the left mouse button, and release on a slice. The source file is left unchanged. The result is written next to it, or into a `DAct` subdirectory when that output mode is selected. If the destination name already exists, a numeric suffix is added: `name (1).ext`, then `name (2).ext`.

## How to use

1. Select a file in File Explorer, or start dragging it. On macOS and Linux the file-manager selection is not read; drop the files on the window, use **Open files**, or pass the paths on the command line.
2. Hold Alt and press the left mouse button. The menu opens under the cursor.
3. Move onto a slice and release the button.
4. Press Alt again while the button is still down to switch between the Convert ring and the Tools ring. Escape closes the menu without running an action.

The trigger key is Alt, Shift, or Ctrl. The default is Alt.

Closing the window does not exit the process. The tray menu item **Quit DAct** does. A second launch while one instance is running forwards its file list to the running process instead of opening another window.

One operation accepts files of a single category. A mix, such as a JPEG and an MP4, is rejected.

## What it does

| File | Convert to | Tools |
|---|---|---|
| Images (PNG, JPG, WEBP, TIFF, BMP, GIF, ICO; HEIC, HEIF, AVIF with FFmpeg) | Any of those, or PDF. Several images become one PDF | Compress · Resize · Rotate 90° · Metadata (strip camera data and GPS) |
| PDF | TXT | Compress (rewrite, drop unused objects) · Split (one file per page) · Rotate 90° · Merge · Extract embedded images · Metadata |
| Text (TXT, MD, CSV, LOG, JSON, XML, HTML) | PDF, DOCX | — |
| Word (DOCX) | TXT | — |
| Audio (MP3, WAV, FLAC, M4A, AAC, AIFF, OGG, OPUS, WMA) | MP3, WAV, FLAC, M4A, AAC, AIFF, OGG, OPUS | — |
| Video (MP4, MOV, MKV, WEBM, AVI, M4V, WMV, MPG, MPEG, FLV, TS, MTS, M2TS, 3GP, ASF, VOB) | MP4, MOV, MKV, WEBM, AVI, M4V | Extract audio to MP3, AAC, or FLAC |

The audio and video rows are recognized extensions and the targets the menu offers. They are not a guarantee that every codec inside those containers will convert. A damaged file or a stream the local FFmpeg build cannot decode fails that file; the rest of a batch continues.

Images, PDF, and text are converted in-process. HEIC, HEIF, AVIF, audio, and video go through a local `ffmpeg` binary. Video is re-encoded: WebM uses VP9 (`libvpx-vp9`, CRF 31) and the other containers use H.264 (`libx264`, preset `medium`, CRF 21). The audio track is copied into MKV, encoded as MP3 for AVI, as Opus for WebM, and as AAC otherwise. DOCX to TXT keeps paragraph text and drops layout, tables, and embedded images. An encrypted PDF is not opened.

Tool outputs carry a suffix on the file stem: `Compressed`, `WIDTHxHEIGHT`, `Rotated`, `Clean`, `Combined` (several images to one PDF), `Merged`, `page N`, `image N`. A straight format conversion keeps the original stem and changes the extension.

## Settings

The settings window sets the trigger key, the output location (next to the original, or a `DAct` subdirectory), JPEG quality (40–100, default 90), the audio quality profile (Low, Medium, High), an optional FFmpeg path, and start at login.

JPEG Compress uses its own quality slider, 30–95, starting at 72. Resize takes a value from 16 to 8000 and one of: percent, long side (default 1280), width, height.

Audio profiles set the bitrate for MP3, AAC, Vorbis, and Opus: Low is 128 kbit/s (Opus 96), Medium is 192 (Opus 128), High is 256 (MP3 320, Opus 192). WAV is PCM. `auto` sample rate, WAV bit depth, and channel layout are the defaults. The same file accepts `audio_sample_rate` as a sample rate in Hz, `audio_wav_bit_depth` as `24` or `32f`, and `audio_channels` as `mono` or `stereo`. Those three keys are not shown in the window.

| | Settings | Log |
|---|---|---|
| Windows | `%APPDATA%\DAct\settings.toml` | `%LOCALAPPDATA%\DAct\log.txt` |
| macOS | `~/Library/Application Support/DAct/settings.toml` | `~/Library/Logs/DAct/log.txt` |
| Linux | `~/.config/DAct/settings.toml` | `~/.local/share/DAct/log.txt` |

`$XDG_CONFIG_HOME` and `$XDG_DATA_HOME` replace the Linux defaults when they are set. The log is renamed to `log.old.txt` after it passes 1 MB.

Start at login runs the executable with `--quiet`. On Windows that is the `DAct` value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`. On macOS it is `~/Library/LaunchAgents/com.dact.app.plist`. On Linux it is `~/.config/autostart/dact.desktop`.

## Command line

```
dact
dact --quiet
dact path\to\file.png
dact convert path\to\file.png --to WEBP
dact convert path\to\clip.mkv --to MP4
dact --selftest
dact --version
```

`--quiet` starts the tray and does not open the window. `convert` prints the output path. `--to` applies to every file on that command. `--selftest` writes a PNG and a TXT file in the temp directory, converts the image to JPG, WEBP, BMP, and PDF, runs resize and compress, converts the text to PDF, and prints `SELFTEST OK`. It does not exercise FFmpeg.

## Build

Rust 1.85 or newer, and a C compiler. `ring` (used by the HTTPS client that downloads FFmpeg on Windows) compiles C code. `Cargo.lock` is part of the repository.

```
cargo build --release
```

The binary is `target/release/dact` (`target\release\dact.exe` on Windows).

FFmpeg is not required to compile or to convert images, PDF, and text.

### Windows

Use the MSVC toolchain, `x86_64-pc-windows-msvc`. Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and select the **Desktop development with C++** workload, which provides MSVC and the Windows SDK. Then:

```
rustup toolchain install stable
rustup default stable
cargo build --release
```

### macOS

Xcode Command Line Tools supply `clang` and the system frameworks (`AppKit`, used by the window and the tray icon).

```
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo build --release
```

Audio and video need a separate FFmpeg install, for example `brew install ffmpeg`. Point **Settings** at the binary, or leave it on `PATH`. The in-app download button is Windows-only.

### Linux

The tray icon links GTK 3, libappindicator (Ayatana or the older libappindicator), and libxdo. Install those before `cargo build`.

Debian and Ubuntu:

```
sudo apt install build-essential pkg-config libgtk-3-dev libxdo-dev libayatana-appindicator3-dev
```

If `libayatana-appindicator3-dev` is not in the archive, use `libappindicator3-dev` instead.

Fedora:

```
sudo dnf install gcc pkgconf-pkg-config gtk3-devel libxdo-devel libayatana-appindicator-gtk3-devel
```

Arch Linux:

```
sudo pacman -S --needed base-devel pkgconf gtk3 xdotool libayatana-appindicator
```

`libappindicator-gtk3` is the alternative to `libayatana-appindicator` when the Ayatana package is absent.

At run time the process loads OpenGL, X11 or Wayland, and the indicator library. The file dialog goes through `xdg-desktop-portal`, so a desktop portal backend has to be installed (`xdg-desktop-portal` and the backend for the session, such as `xdg-desktop-portal-gtk` or `xdg-desktop-portal-gnome`). Install FFmpeg from the distribution and set the path in **Settings**, or put `ffmpeg` on `PATH`. The in-app download button is Windows-only.

### Check

```
cargo test
cargo run --release -- --selftest
```

`cargo test` covers extension classification and output-name uniqueness. `--selftest` covers the image and text conversions listed above.

## FFmpeg

Search order: the path in settings, then the copy downloaded by the app, then `ffmpeg` on `PATH`, then a short list of common install locations (`~/bin`, `~/ffmpeg/bin`, WinGet links, `C:\ffmpeg\bin`, `/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`).

On Windows, **Get FFmpeg** downloads the essentials build from <https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip> and extracts `ffmpeg.exe` and `ffprobe.exe` into `%LOCALAPPDATA%\DAct\ffmpeg`. Nothing is downloaded at startup. That build is GPL, because it includes encoders such as libx264 and LAME. It is not part of this repository, and the MIT license on DAct does not cover it. Shipping an installer that contains that binary means complying with the FFmpeg license separately.

## License

[MIT](LICENSE). Copyright (c) 2026 Roma Kosyachenko.

Crate licenses are the licenses of the crates recorded in `Cargo.lock`. FFmpeg, when installed or downloaded, stays under its own license.
