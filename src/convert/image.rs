use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, ImageFormat, RgbaImage};

use crate::error::{DactError, Result};
use crate::paths::{atomic_write, sibling};
use crate::settings::Settings;

pub const IMAGE_TARGETS: &[&str] = &["PNG", "JPG", "WEBP", "TIFF", "BMP", "PDF", "ICO", "GIF"];

pub fn targets_for(src: &Path) -> Vec<String> {
    let cur = src
        .extension()
        .and_then(|e| e.to_str())
        .map(crate::category::normalize_label)
        .unwrap_or_default();
    IMAGE_TARGETS
        .iter()
        .filter(|t| **t != cur.as_str())
        .map(|s| (*s).to_string())
        .collect()
}

pub fn open_image(path: &Path) -> Result<DynamicImage> {
    let img = image::open(path).map_err(|_| DactError::Unsupported)?;
    Ok(match img {
        DynamicImage::ImageRgba8(_) | DynamicImage::ImageRgb8(_) | DynamicImage::ImageLuma8(_) => {
            img
        }
        other => DynamicImage::ImageRgba8(other.to_rgba8()),
    })
}

/// Built-in decode first; HEIC/AVIF and odd codecs fall back to local FFmpeg.
pub fn open_image_any(path: &Path, settings: &Settings) -> Result<DynamicImage> {
    match open_image(path) {
        Ok(img) => Ok(img),
        Err(first) => decode_via_ffmpeg(path, settings).map_err(|_| first),
    }
}

fn decode_via_ffmpeg(path: &Path, settings: &Settings) -> Result<DynamicImage> {
    let ff = crate::ffmpeg::Ffmpeg::resolve(settings)?;
    let tmp = crate::paths::unique_path(path.with_file_name(format!(
        ".~{}.dact.png",
        path.file_stem().and_then(|s| s.to_str()).unwrap_or("img")
    )));
    let result = (|| {
        ff.convert_file(path, &tmp, &[])?;
        open_image(&tmp)
    })();
    let _ = std::fs::remove_file(&tmp);
    result
}

fn flatten(img: &DynamicImage) -> DynamicImage {
    match img {
        DynamicImage::ImageRgba8(rgba) => DynamicImage::ImageRgb8(flatten_rgba(rgba)),
        other if other.color().has_alpha() => {
            DynamicImage::ImageRgb8(flatten_rgba(&other.to_rgba8()))
        }
        other => other.to_rgb8().into(),
    }
}

fn flatten_rgba(rgba: &RgbaImage) -> image::RgbImage {
    let mut out = image::RgbImage::new(rgba.width(), rgba.height());
    for (x, y, p) in rgba.enumerate_pixels() {
        let a = p[3] as f32 / 255.0;
        let r = (p[0] as f32 * a + 255.0 * (1.0 - a)).round() as u8;
        let g = (p[1] as f32 * a + 255.0 * (1.0 - a)).round() as u8;
        let b = (p[2] as f32 * a + 255.0 * (1.0 - a)).round() as u8;
        out.put_pixel(x, y, image::Rgb([r, g, b]));
    }
    out
}

pub fn encode_image(img: &DynamicImage, target: &str, quality: u8) -> Result<Vec<u8>> {
    let target = target.to_ascii_uppercase();
    let mut buf = Vec::new();
    match target.as_str() {
        "JPG" | "JPEG" => {
            let rgb = flatten(img).to_rgb8();
            let mut enc =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality.clamp(30, 100));
            enc.encode_image(&rgb)
                .map_err(|e| DactError::user(e.to_string()))?;
        }
        "PNG" => {
            img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
                .map_err(|e| DactError::user(e.to_string()))?;
        }
        "WEBP" => {
            let rgba = img.to_rgba8();
            let enc = image::codecs::webp::WebPEncoder::new_lossless(&mut buf);
            enc.encode(
                rgba.as_raw(),
                rgba.width(),
                rgba.height(),
                ExtendedColorType::Rgba8,
            )
            .map_err(|e| DactError::user(e.to_string()))?;
        }
        "TIFF" => {
            img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Tiff)
                .map_err(|e| DactError::user(e.to_string()))?;
        }
        "BMP" => {
            flatten(img)
                .write_to(&mut Cursor::new(&mut buf), ImageFormat::Bmp)
                .map_err(|e| DactError::user(e.to_string()))?;
        }
        "GIF" => {
            img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Gif)
                .map_err(|e| DactError::user(e.to_string()))?;
        }
        "ICO" => {
            let side = 256.min(img.width().max(img.height()));
            let work = img.resize(side, side, FilterType::Lanczos3).to_rgba8();
            DynamicImage::ImageRgba8(work)
                .write_to(&mut Cursor::new(&mut buf), ImageFormat::Ico)
                .map_err(|e| DactError::user(e.to_string()))?;
        }
        "PDF" => {
            buf = crate::convert::pdf::image_bytes_to_pdf(&flatten(img).to_rgb8())?;
        }
        other => return Err(DactError::user(format!("Unknown image target {other}"))),
    }
    Ok(buf)
}

pub fn image_to(src: &Path, target: &str, settings: &Settings) -> Result<PathBuf> {
    let img = open_image_any(src, settings)?;
    let ext = match target.to_ascii_uppercase().as_str() {
        "JPG" | "JPEG" => "jpg".to_string(),
        "TIFF" => "tiff".to_string(),
        other => other.to_ascii_lowercase(),
    };
    let out = sibling(src, "", &ext, settings.output)?;
    let bytes = encode_image(&img, target, settings.jpg_quality())?;
    atomic_write(&out, &bytes)?;
    Ok(out)
}

pub fn images_to_single_pdf(srcs: &[PathBuf], settings: &Settings) -> Result<PathBuf> {
    if srcs.is_empty() {
        return Err(DactError::user("No images to combine."));
    }
    let mut pages = Vec::new();
    for src in srcs {
        pages.push(flatten(&open_image_any(src, settings)?).to_rgb8());
    }
    let out = sibling(&srcs[0], "Combined", "pdf", settings.output)?;
    let bytes = crate::convert::pdf::images_to_pdf(&pages)?;
    atomic_write(&out, &bytes)?;
    Ok(out)
}

pub fn compress_image(src: &Path, quality: u8, settings: &Settings) -> Result<PathBuf> {
    let img = open_image_any(src, settings)?;
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg")
        .to_ascii_lowercase();
    let (target, out_ext) = match ext.as_str() {
        "png" => ("PNG", "png"),
        "webp" => ("WEBP", "webp"),
        "jpg" | "jpeg" => ("JPG", "jpg"),
        _ => ("JPG", "jpg"),
    };
    let out = sibling(src, "Compressed", out_ext, settings.output)?;
    if target == "PNG" {
        let colors = (quality as u32 * 256 / 100).clamp(16, 256) as usize;
        let quantized = quantize_rgba(&img.to_rgba8(), colors);
        let mut buf = Vec::new();
        DynamicImage::ImageRgba8(quantized)
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .map_err(|e| DactError::user(e.to_string()))?;
        atomic_write(&out, &buf)?;
    } else {
        atomic_write(&out, &encode_image(&img, target, quality)?)?;
    }
    Ok(out)
}

fn quantize_rgba(img: &RgbaImage, colors: usize) -> RgbaImage {
    // Simple median-cut-ish fallback: reduce channel resolution.
    let shift = match colors {
        0..=16 => 5,
        17..=32 => 4,
        33..=64 => 3,
        _ => 2,
    };
    let mask = !((1u8 << shift) - 1);
    let mut out = img.clone();
    for p in out.pixels_mut() {
        p[0] &= mask;
        p[1] &= mask;
        p[2] &= mask;
    }
    out
}

pub fn resize_image(src: &Path, width: u32, height: u32, settings: &Settings) -> Result<PathBuf> {
    let img = open_image_any(src, settings)?;
    let resized = img.resize_exact(width.max(1), height.max(1), FilterType::Lanczos3);
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png");
    let out = sibling(src, &format!("{width}x{height}"), ext, settings.output)?;
    let target = crate::category::normalize_label(ext);
    let target = if matches!(target.as_str(), "HEIC" | "AVIF") {
        "PNG".into()
    } else {
        target
    };
    atomic_write(&out, &encode_image(&resized, &target, settings.jpg_quality())?)?;
    Ok(out)
}

pub fn resize_dims(size: (u32, u32), mode: &str, value: u32) -> (u32, u32) {
    let (w, h) = size;
    let k = match mode {
        "pct" => value as f32 / 100.0,
        "long" => value as f32 / w.max(h) as f32,
        "w" => value as f32 / w as f32,
        _ => value as f32 / h as f32,
    };
    (
        ((w as f32 * k).round() as u32).max(1),
        ((h as f32 * k).round() as u32).max(1),
    )
}

pub fn rotate_image(src: &Path, settings: &Settings) -> Result<PathBuf> {
    let img = open_image_any(src, settings)?.rotate90();
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png");
    let out = sibling(src, "Rotated", ext, settings.output)?;
    let target = crate::category::normalize_label(ext);
    atomic_write(&out, &encode_image(&img, &target, settings.jpg_quality())?)?;
    Ok(out)
}

pub fn strip_metadata(src: &Path, settings: &Settings) -> Result<PathBuf> {
    let img = open_image_any(src, settings)?;
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png");
    let out = sibling(src, "Clean", ext, settings.output)?;
    let target = crate::category::normalize_label(ext);
    atomic_write(&out, &encode_image(&img, &target, settings.jpg_quality())?)?;
    Ok(out)
}

pub fn read_metadata(src: &Path) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    if let Ok((w, h)) = image::image_dimensions(src) {
        rows.push(("Size".into(), format!("{w} × {h}")));
    }
    if let Ok(bytes) = fs::read(src) {
        let mut reader = std::io::Cursor::new(bytes);
        if let Ok(exif) = exif::Reader::new().read_from_container(&mut reader) {
            for f in exif.fields() {
                let tag = f.tag.to_string();
                let value = f.display_value().with_unit(&exif).to_string();
                if matches!(
                    tag.as_str(),
                    "Make"
                        | "Model"
                        | "DateTime"
                        | "DateTimeOriginal"
                        | "GPSLatitude"
                        | "GPSLongitude"
                        | "GPSAltitude"
                        | "Orientation"
                        | "Software"
                        | "Artist"
                        | "Copyright"
                ) {
                    rows.push((tag, value));
                }
            }
        }
    }
    if rows.len() == 1 {
        rows.push(("Metadata".into(), "none stored".into()));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn sample(dir: &Path) -> PathBuf {
        let p = dir.join("t.png");
        let img = image::RgbImage::from_pixel(32, 24, image::Rgb([200, 80, 40]));
        DynamicImage::ImageRgb8(img)
            .save(&p)
            .unwrap();
        p
    }

    #[test]
    fn converts_png_to_jpg() {
        let dir = tempfile::tempdir().unwrap();
        let src = sample(dir.path());
        let out = image_to(&src, "JPG", &Settings::default()).unwrap();
        assert!(out.exists());
        assert_eq!(out.extension().unwrap(), "jpg");
        assert_ne!(out, src);
    }

    #[test]
    fn resize_keeps_aspect_from_width() {
        assert_eq!(resize_dims((800, 400), "w", 400), (400, 200));
        assert_eq!(resize_dims((100, 50), "pct", 50), (50, 25));
    }

    #[test]
    fn png_to_pdf_and_back_stays_beside_source() {
        let dir = tempfile::tempdir().unwrap();
        let src = sample(dir.path());
        let pdf = image_to(&src, "PDF", &Settings::default()).unwrap();
        assert_eq!(pdf.extension().unwrap(), "pdf");
        assert!(pdf.exists());
        assert_ne!(pdf, src);
        assert!(src.exists());
    }
}
