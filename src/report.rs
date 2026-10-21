use std::path::{Path, PathBuf};

use crate::category::{category, Category};
use crate::paths::{file_size, human_size};

#[derive(Debug, Clone)]
pub struct ResultCard {
    pub title: String,
    pub detail: String,
    pub good: String,
    pub outs: Vec<PathBuf>,
    pub srcs: Vec<PathBuf>,
}

pub fn summarize(action: &str, srcs: &[PathBuf], outs: &[PathBuf], extra: &str) -> ResultCard {
    let before: u64 = srcs.iter().map(file_size).sum();
    let after: u64 = outs.iter().map(file_size).sum();
    let (kind, name) = action.split_once(':').unwrap_or(("", action));
    if outs.is_empty() {
        return ResultCard {
            title: action.to_string(),
            detail: if extra.is_empty() {
                crate::i18n::nothing_written().into()
            } else {
                extra.into()
            },
            good: String::new(),
            outs: vec![],
            srcs: srcs.to_vec(),
        };
    }
    let o0 = &outs[0];
    if kind == "convert" || kind == "media" || kind == "extract" {
        let title = if srcs.len() > 1 && outs.len() == 1 {
            crate::i18n::files_to_one(srcs.len(), name)
        } else if outs.len() > 1 && srcs.len() == 1 {
            crate::i18n::pages_to_files(outs.len(), name)
        } else if outs.len() > 1 {
            crate::i18n::files_to_fmt(outs.len(), name)
        } else if let Some(src) = srcs.first() {
            crate::i18n::ext_to_fmt(
                &src.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or(crate::i18n::fallback_file())
                    .to_ascii_uppercase(),
                name,
            )
        } else {
            crate::i18n::converted_to(name)
        };
        let mut bits = vec![if outs.len() == 1 {
            file_name(o0)
        } else {
            crate::i18n::files_n(outs.len())
        }];
        if category(o0) == Some(Category::Image) {
            if let Some((w, h)) = image_dims(o0) {
                bits.push(format!("{w}×{h}"));
            }
        }
        if before > 0 && before != after {
            bits.push(format!("{} → {}", human_size(before), human_size(after)));
        } else {
            bits.push(human_size(after));
        }
        let (_, good) = size_change(before, after);
        return ResultCard {
            title,
            detail: bits.join(" · "),
            good,
            outs: outs.to_vec(),
            srcs: srcs.to_vec(),
        };
    }

    if name == "Compress" {
        let (sizes, verdict) = size_change(before, after);
        let detail = if extra.is_empty() {
            sizes
        } else {
            format!("{sizes} · {extra}")
        };
        return ResultCard {
            title: if srcs.len() > 1 {
                crate::i18n::compressed_n(srcs.len())
            } else {
                crate::i18n::compressed_title().into()
            },
            detail,
            good: verdict,
            outs: outs.to_vec(),
            srcs: srcs.to_vec(),
        };
    }

    if matches!(name, "Resize" | "Rotate") {
        let title = if name == "Resize" {
            crate::i18n::resized_title()
        } else {
            crate::i18n::rotated_title()
        };
        if outs.len() > 1 {
            return ResultCard {
                title: if name == "Resize" {
                    crate::i18n::resized_n(outs.len())
                } else {
                    crate::i18n::rotated_n(outs.len())
                },
                detail: format!("{} → {}", human_size(before), human_size(after)),
                good: String::new(),
                outs: outs.to_vec(),
                srcs: srcs.to_vec(),
            };
        }
        let d0 = srcs.first().and_then(|p| image_dims(p));
        let d1 = image_dims(o0);
        let mut detail = match (d0, d1) {
            (Some((a, b)), Some((c, d))) if (a, b) != (c, d) => format!("{a}×{b} → {c}×{d}"),
            (None, Some((c, d))) | (Some((c, d)), None) => format!("{c}×{d}"),
            _ => String::new(),
        };
        if !extra.is_empty() {
            if !detail.is_empty() {
                detail.push_str(" · ");
            }
            detail.push_str(extra);
        }
        if !detail.is_empty() {
            detail.push_str(" · ");
        }
        detail.push_str(&file_name(o0));
        return ResultCard {
            title: title.into(),
            detail,
            good: String::new(),
            outs: outs.to_vec(),
            srcs: srcs.to_vec(),
        };
    }

    let title = match name {
        "Metadata" => crate::i18n::metadata_removed(),
        "Split" => {
            return ResultCard {
                title: crate::i18n::split_into(outs.len()),
                detail: format!("{} …", file_name(o0)),
                good: String::new(),
                outs: outs.to_vec(),
                srcs: srcs.to_vec(),
            }
        }
        "Merge" => crate::i18n::merged_pdfs(),
        "Extract" => {
            return ResultCard {
                title: crate::i18n::extracted_images(outs.len()),
                detail: human_size(after),
                good: String::new(),
                outs: outs.to_vec(),
                srcs: srcs.to_vec(),
            }
        }
        other => other,
    };

    ResultCard {
        title: title.into(),
        detail: format!("{} · {}", file_name(o0), human_size(after)),
        good: if name == "Metadata" {
            crate::i18n::private_copy().into()
        } else {
            String::new()
        },
        outs: outs.to_vec(),
        srcs: srcs.to_vec(),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

fn size_change(before: u64, after: u64) -> (String, String) {
    let text = format!("{} → {}", human_size(before), human_size(after));
    if before == 0 {
        return (text, String::new());
    }
    let pct = (1.0 - after as f64 / before as f64) * 100.0;
    if pct >= 1.0 {
        (text, crate::i18n::pct_smaller(pct))
    } else if pct <= -1.0 {
        (text, crate::i18n::already_small().into())
    } else {
        (text, crate::i18n::same_size().into())
    }
}

fn image_dims(path: &Path) -> Option<(u32, u32)> {
    image::image_dimensions(path).ok()
}
