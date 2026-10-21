use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{DactError, Result};
use crate::paths::{atomic_write, sibling};
use crate::settings::Settings;

pub fn read_text(src: &Path) -> Result<String> {
    let raw = fs::read(src).map_err(DactError::from_io)?;
    for enc in [encoding_guess(&raw)] {
        if let Ok(s) = decode(&raw, enc) {
            return Ok(s);
        }
    }
    Ok(String::from_utf8_lossy(&raw).into_owned())
}

fn encoding_guess(raw: &[u8]) -> &'static str {
    if raw.starts_with(&[0xFF, 0xFE]) || raw.starts_with(&[0xFE, 0xFF]) {
        "utf16"
    } else {
        "utf8"
    }
}

fn decode(raw: &[u8], enc: &str) -> Result<String> {
    match enc {
        "utf16" => {
            if raw.starts_with(&[0xFF, 0xFE]) {
                let units: Vec<u16> = raw[2..]
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16(&units).map_err(|_| DactError::Unsupported)
            } else if raw.starts_with(&[0xFE, 0xFF]) {
                let units: Vec<u16> = raw[2..]
                    .chunks_exact(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16(&units).map_err(|_| DactError::Unsupported)
            } else {
                String::from_utf8(raw.to_vec()).map_err(|_| DactError::Unsupported)
            }
        }
        _ => match std::str::from_utf8(raw) {
            Ok(s) => Ok(s.trim_start_matches('\u{feff}').to_string()),
            Err(_) => {
                // Windows-1252-ish fallback: keep bytes as Latin-1
                Ok(raw.iter().map(|&b| b as char).collect())
            }
        },
    }
}

pub fn text_to_pdf(src: &Path, settings: &Settings) -> Result<PathBuf> {
    let text = read_text(src)?;
    let out = sibling(src, "", "pdf", settings.output)?;
    atomic_write(&out, &crate::convert::pdf::text_to_pdf_bytes(&text)?)?;
    Ok(out)
}

pub fn text_to_docx(src: &Path, settings: &Settings) -> Result<PathBuf> {
    let text = read_text(src)?;
    let mut docx = docx_rs::Docx::new();
    for line in text.lines() {
        docx = docx.add_paragraph(docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_text(line)));
    }
    let out = sibling(src, "", "docx", settings.output)?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    docx.build()
        .pack(&mut cursor)
        .map_err(|e| DactError::user(e.to_string()))?;
    atomic_write(&out, &cursor.into_inner())?;
    Ok(out)
}

pub fn docx_to_txt(src: &Path, settings: &Settings) -> Result<PathBuf> {
    let text = extract_docx_text(src)?;
    let out = sibling(src, "", "txt", settings.output)?;
    atomic_write(&out, text.as_bytes())?;
    Ok(out)
}

pub fn extract_docx_text(src: &Path) -> Result<String> {
    let file = fs::File::open(src).map_err(DactError::from_io)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| DactError::user(e.to_string()))?;
    let mut xml = archive
        .by_name("word/document.xml")
        .map_err(|_| DactError::user("This Word file is damaged or not a .docx."))?;
    let mut raw = String::new();
    std::io::Read::read_to_string(&mut xml, &mut raw).map_err(DactError::from_io)?;
    Ok(strip_docx_xml(&raw))
}

fn strip_docx_xml(xml: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    let bytes = xml.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'<' {
            if xml[i..].starts_with("<w:tab") {
                out.push('\t');
            } else if xml[i..].starts_with("<w:br") || xml[i..].starts_with("<w:p ") || xml[i..].starts_with("<w:p>") {
                if !out.ends_with('\n') && !out.is_empty() {
                    out.push('\n');
                }
            }
            if let Some(end) = xml[i..].find('>') {
                i += end + 1;
                continue;
            }
            break;
        }
        let rest = &xml[i..];
        let ch = rest.chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_simple_docx_xml() {
        let xml = r#"<w:document><w:p><w:r><w:t>Hello</w:t></w:r></w:p><w:p><w:r><w:t>World</w:t></w:r></w:p></w:document>"#;
        let text = strip_docx_xml(xml);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn text_becomes_pdf_without_touching_source() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("note.txt");
        std::fs::write(&src, "hello from dact").unwrap();
        let out = text_to_pdf(&src, &crate::settings::Settings::default()).unwrap();
        assert!(out.exists());
        assert_eq!(out.extension().unwrap(), "pdf");
        assert_eq!(std::fs::read_to_string(&src).unwrap(), "hello from dact");
    }
}
