use std::path::{Path, PathBuf};

use crate::category::{self, category, Category};
use crate::error::{DactError, Result};
use crate::settings::Settings;

pub mod image;
pub mod media;
pub mod pdf;
pub mod text;

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub hint: String,
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct Ring {
    pub name: String,
    pub items: Vec<MenuItem>,
}

#[derive(Debug, Clone)]
pub struct JobProgress {
    pub completed: usize,
    pub total: usize,
    pub current: PathBuf,
    pub file_frac: f32,
}

impl JobProgress {
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        ((self.completed as f32 + self.file_frac.clamp(0.0, 1.0)) / self.total as f32)
            .clamp(0.0, 1.0)
    }

    pub fn detail(&self) -> String {
        let name = self
            .current
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".into());
        let pct = (self.fraction() * 100.0).round() as u32;
        let n = (self.completed + 1).min(self.total.max(1));
        if self.total > 1 {
            format!("{n} / {}  ·  {name}  ·  {pct}%", self.total)
        } else if pct > 0 {
            format!("{name}  ·  {pct}%")
        } else {
            name
        }
    }
}

pub fn build_rings(paths: &[PathBuf], settings: &Settings) -> Vec<Ring> {
    let Some(first) = paths.first() else {
        return vec![];
    };
    if category::uniform(paths).is_none() {
        return vec![Ring {
            name: "One type only".into(),
            items: vec![],
        }];
    }
    let Some(cat) = category(first) else {
        return vec![Ring {
            name: "Unsupported".into(),
            items: vec![],
        }];
    };
    let same = paths.to_vec();

    match cat {
        Category::Audio => vec![Ring {
            name: "Audio".into(),
            items: media::audio_targets(first)
                .into_iter()
                .map(|n| MenuItem {
                    hint: "Local FFmpeg".into(),
                    key: format!("media:{n}"),
                    label: n,
                })
                .collect(),
        }],
        Category::Video => {
            let mut rings = vec![Ring {
                name: "Video".into(),
                items: media::video_targets(first)
                    .into_iter()
                    .map(|n| MenuItem {
                        hint: "Local FFmpeg".into(),
                        key: format!("media:{n}"),
                        label: n,
                    })
                    .collect(),
            }];
            rings.push(Ring {
                name: "Extract".into(),
                items: ["MP3", "AAC", "FLAC"]
                    .into_iter()
                    .map(|n| MenuItem {
                        label: n.into(),
                        hint: "Audio only".into(),
                        key: format!("extract:{n}"),
                    })
                    .collect(),
            });
            let _ = settings;
            rings
        }
        Category::Image => {
            let mut convert = Vec::new();
            for t in image::targets_for(first) {
                let hint = if t == "PDF" && same.len() > 1 {
                    format!("{} images → 1 PDF", same.len())
                } else {
                    format_hint(&t).into()
                };
                convert.push(MenuItem {
                    label: t.clone(),
                    hint,
                    key: format!("convert:{t}"),
                });
            }
            vec![
                Ring {
                    name: "Convert".into(),
                    items: convert,
                },
                Ring {
                    name: "Tools".into(),
                    items: tool_items(&[
                        ("Compress", "Shrink file size"),
                        ("Resize", "Change dimensions"),
                        ("Rotate", "Turn 90°"),
                        ("Metadata", "Strip camera / GPS"),
                    ]),
                },
            ]
        }
        Category::Pdf => {
            let mut tools = vec![
                ("Compress", "Rewrite, drop unused objects"),
                ("Split", "One PDF per page"),
                ("Rotate", "Turn pages 90°"),
            ];
            if same.len() > 1 {
                tools.insert(3, ("Merge", "Join into one PDF"));
            }
            tools.extend([
                ("Extract", "Save embedded images"),
                ("Metadata", "Private copy"),
            ]);
            vec![
                Ring {
                    name: "Convert".into(),
                    items: pdf::pdf_targets()
                        .into_iter()
                        .map(|t| MenuItem {
                            hint: "Extract text".into(),
                            key: format!("convert:{t}"),
                            label: t,
                        })
                        .collect(),
                },
                Ring {
                    name: "Tools".into(),
                    items: tool_items(&tools),
                },
            ]
        }
        Category::Text => vec![Ring {
            name: "Convert".into(),
            items: ["PDF", "DOCX"]
                .into_iter()
                .map(|t| MenuItem {
                    hint: format_hint(t).into(),
                    key: format!("convert:{t}"),
                    label: t.into(),
                })
                .collect(),
        }],
        Category::Docx => vec![Ring {
            name: "Convert".into(),
            items: vec![MenuItem {
                label: "TXT".into(),
                hint: "Plain text".into(),
                key: "convert:TXT".into(),
            }],
        }],
    }
}

fn tool_items(items: &[(&str, &str)]) -> Vec<MenuItem> {
    items
        .iter()
        .map(|(label, hint)| MenuItem {
            label: (*label).into(),
            hint: (*hint).into(),
            key: format!("tool:{label}"),
        })
        .collect()
}

fn format_hint(target: &str) -> &'static str {
    match target {
        "JPG" => "Small, universal",
        "PNG" => "Lossless · transparency",
        "WEBP" => "Small for the web",
        "TIFF" => "Print · archival",
        "BMP" => "Uncompressed",
        "PDF" => "Document",
        "ICO" => "Windows icon",
        "GIF" => "Legacy web",
        "DOCX" => "Editable Word",
        "TXT" => "Plain text",
        _ => "",
    }
}

pub fn run_action(
    key: &str,
    paths: &[PathBuf],
    settings: &Settings,
    extra: &ActionExtra,
) -> Result<Vec<PathBuf>> {
    run_action_with_progress(key, paths, settings, extra, None)
}

pub fn run_action_with_progress(
    key: &str,
    paths: &[PathBuf],
    settings: &Settings,
    extra: &ActionExtra,
    mut on_progress: Option<&mut dyn FnMut(JobProgress)>,
) -> Result<Vec<PathBuf>> {
    let Some(first) = paths.first() else {
        return Err(DactError::user("No files."));
    };
    let Some(cat) = category::uniform(paths) else {
        if paths.iter().any(|p| category(p).is_none()) {
            return Err(DactError::Unsupported);
        }
        return Err(DactError::user(
            "Select files of one type — images, or video, or audio, or PDF.",
        ));
    };
    let (kind, name) = key.split_once(':').unwrap_or(("", key));
    let total = paths.len();

    let mut tick = |completed: usize, current: &Path, file_frac: f32| {
        if let Some(cb) = on_progress.as_mut() {
            cb(JobProgress {
                completed,
                total,
                current: current.to_path_buf(),
                file_frac,
            });
        }
    };

    match kind {
        "media" | "extract" => {
            let media_kind = if kind == "extract" || cat == Category::Audio {
                Category::Audio
            } else {
                Category::Video
            };
            let mut outs = Vec::new();
            let mut errors = Vec::new();
            for (i, p) in paths.iter().enumerate() {
                tick(i, p, 0.0);
                match media::convert_media(p, name, media_kind, kind == "extract", settings, &mut |f| {
                    tick(i, p, f);
                }) {
                    Ok(o) => outs.push(o),
                    Err(e) => errors.push(format!("{}: {e}", p.display())),
                }
                tick(i + 1, p, 0.0);
            }
            if outs.is_empty() {
                return Err(DactError::user(errors.join("; ")));
            }
            Ok(outs)
        }
        "convert" => {
            if cat == Category::Image && name == "PDF" && paths.len() > 1 {
                tick(0, first, 0.0);
                let out = image::images_to_single_pdf(paths, settings)?;
                tick(paths.len(), first, 1.0);
                return Ok(vec![out]);
            }
            let mut outs = Vec::new();
            for (i, p) in paths.iter().enumerate() {
                tick(i, p, 0.0);
                outs.extend(convert_one(p, cat, name, settings)?);
                tick(i + 1, p, 0.0);
            }
            Ok(outs)
        }
        "tool" => {
            tick(0, first, 0.0);
            let outs = run_tool(name, paths, cat, settings, extra)?;
            tick(paths.len(), paths.last().unwrap_or(first), 1.0);
            Ok(outs)
        }
        _ => Err(DactError::user("Unknown action.")),
    }
}

fn convert_one(src: &Path, cat: Category, target: &str, settings: &Settings) -> Result<Vec<PathBuf>> {
    match (cat, target) {
        (Category::Image, _) => Ok(vec![image::image_to(src, target, settings)?]),
        (Category::Pdf, "TXT") => Ok(vec![pdf::pdf_to_txt(src, settings)?]),
        (Category::Text, "PDF") => Ok(vec![text::text_to_pdf(src, settings)?]),
        (Category::Text, "DOCX") => Ok(vec![text::text_to_docx(src, settings)?]),
        (Category::Docx, "TXT") => Ok(vec![text::docx_to_txt(src, settings)?]),
        _ => Err(DactError::user(format!(
            "Can't convert this file to {target}."
        ))),
    }
}

fn run_tool(
    name: &str,
    paths: &[PathBuf],
    cat: Category,
    settings: &Settings,
    extra: &ActionExtra,
) -> Result<Vec<PathBuf>> {
    match (cat, name) {
        (Category::Image, "Compress") => {
            let q = extra.quality.unwrap_or(72);
            let mut outs = Vec::new();
            for p in paths {
                outs.push(image::compress_image(p, q, settings)?);
            }
            Ok(outs)
        }
        (Category::Image, "Resize") => {
            let (mode, value) = extra
                .resize
                .clone()
                .unwrap_or_else(|| ("long".into(), 1280));
            let mut outs = Vec::new();
            for p in paths {
                let dims = ::image::image_dimensions(p).map_err(|_| DactError::Unsupported)?;
                let (w, h) = image::resize_dims(dims, &mode, value);
                outs.push(image::resize_image(p, w, h, settings)?);
            }
            Ok(outs)
        }
        (Category::Image, "Rotate") => {
            let mut outs = Vec::new();
            for p in paths {
                outs.push(image::rotate_image(p, settings)?);
            }
            Ok(outs)
        }
        (Category::Image, "Metadata") => {
            let mut outs = Vec::new();
            for p in paths {
                outs.push(image::strip_metadata(p, settings)?);
            }
            Ok(outs)
        }
        (Category::Pdf, "Split") => {
            let mut outs = Vec::new();
            for p in paths {
                outs.extend(pdf::split_pdf(p, settings)?);
            }
            Ok(outs)
        }
        (Category::Pdf, "Merge") => Ok(vec![pdf::merge_pdfs(paths, settings)?]),
        (Category::Pdf, "Rotate") => {
            let mut outs = Vec::new();
            for p in paths {
                outs.push(pdf::rotate_pdf(p, settings)?);
            }
            Ok(outs)
        }
        (Category::Pdf, "Extract") => {
            let mut outs = Vec::new();
            for p in paths {
                outs.extend(pdf::extract_pdf_images(p, settings)?);
            }
            Ok(outs)
        }
        (Category::Pdf, "Compress") => {
            let mut outs = Vec::new();
            for p in paths {
                outs.push(pdf::compress_pdf(p, settings)?);
            }
            Ok(outs)
        }
        (Category::Pdf, "Metadata") => {
            let mut outs = Vec::new();
            for p in paths {
                outs.push(strip_pdf_metadata(p, settings)?);
            }
            Ok(outs)
        }
        _ => Err(DactError::user(format!("Tool {name} is not available."))),
    }
}

fn strip_pdf_metadata(src: &Path, settings: &Settings) -> Result<PathBuf> {
    let mut doc = lopdf::Document::load(src).map_err(|e| DactError::user(e.to_string()))?;
    doc.trailer.remove(b"Info");
    if let Ok(root) = doc.trailer.get_mut(b"Root") {
        if let lopdf::Object::Dictionary(dict) = root {
            dict.remove(b"Metadata");
        }
    }
    let out = crate::paths::sibling(src, "Clean", "pdf", settings.output)?;
    doc.save(&out)
        .map_err(|e| DactError::user(e.to_string()))?;
    Ok(out)
}

#[derive(Debug, Clone, Default)]
pub struct ActionExtra {
    pub quality: Option<u8>,
    pub resize: Option<(String, u32)>,
}
