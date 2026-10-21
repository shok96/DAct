use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use image::RgbImage;
use lopdf::{dictionary, Document, Object, ObjectId, Stream};

use crate::error::{DactError, Result};
use crate::paths::{atomic_write, sibling};
use crate::settings::Settings;

const PAGE_PT_W: f32 = 595.0;
const PAGE_PT_H: f32 = 842.0;

pub fn pdf_targets() -> Vec<String> {
    vec!["TXT".into()]
}

pub fn image_bytes_to_pdf(rgb: &RgbImage) -> Result<Vec<u8>> {
    images_to_pdf(std::slice::from_ref(rgb))
}

pub fn images_to_pdf(pages: &[RgbImage]) -> Result<Vec<u8>> {
    if pages.is_empty() {
        return Err(DactError::user(crate::i18n::err_no_pages()));
    }
    let mut pdf = Vec::new();
    let mut xref: Vec<u64> = vec![0];
    let mut write_obj = |pdf: &mut Vec<u8>, body: &[u8]| {
        xref.push(pdf.len() as u64);
        pdf.extend_from_slice(body);
        if !body.ends_with(b"\n") {
            pdf.push(b'\n');
        }
    };

    pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let n = pages.len();
    let mut jpeg_objs = Vec::new();
    let mut content_objs = Vec::new();
    let mut page_objs = Vec::new();

    for (i, page) in pages.iter().enumerate() {
        let mut jpeg = Vec::new();
        {
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 90);
            enc.encode_image(page)
                .map_err(|e| DactError::user(e.to_string()))?;
        }
        let w = page.width();
        let h = page.height();
        let (pw, ph) = fit_page(w, h);
        jpeg_objs.push((i, jpeg, w, h, pw, ph));
    }

    // Object layout:
    // 1 Catalog
    // 2 Pages
    // then for each page: Image, Content, Page
    // last: Font (for text PDFs we add separately)
    write_obj(
        &mut pdf,
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
    );

    let first_page_obj = 3 + n as u32 * 3;
    // We'll write pages kids after we know ids.
    // Image i -> 3 + i*3
    // Content i -> 4 + i*3
    // Page i -> 5 + i*3
    let mut kids = String::from("/Kids [");
    for i in 0..n {
        let page_id = 5 + i * 3;
        kids.push_str(&format!("{page_id} 0 R "));
        page_objs.push(page_id);
        content_objs.push(4 + i * 3);
    }
    kids.push_str(&format!("] /Count {n}"));

    // Placeholder for object 2; we'll rewrite via full build below using a simpler writer.
    let _ = (first_page_obj, kids, write_obj, jpeg_objs, content_objs, page_objs);
    write_simple_image_pdf(pages)
}

fn fit_page(w: u32, h: u32) -> (f32, f32) {
    let max_w = PAGE_PT_W - 48.0;
    let max_h = PAGE_PT_H - 48.0;
    let scale = (max_w / w as f32).min(max_h / h as f32).min(1.0);
    (w as f32 * scale, h as f32 * scale)
}

fn write_simple_image_pdf(pages: &[RgbImage]) -> Result<Vec<u8>> {
    let n = pages.len();
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(Vec::new()); // 1-based

    let mut jpegs = Vec::new();
    for page in pages {
        let mut jpeg = Vec::new();
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 90);
        enc.encode_image(page)
            .map_err(|e| DactError::user(e.to_string()))?;
        jpegs.push((jpeg, page.width(), page.height()));
    }

    // ids: 1 catalog, 2 pages, then triples
    let catalog_id = 1u32;
    let pages_id = 2u32;
    let mut next = 3u32;
    let mut page_ids = Vec::new();
    let mut image_ids = Vec::new();
    let mut content_ids = Vec::new();
    for _ in 0..n {
        image_ids.push(next);
        next += 1;
        content_ids.push(next);
        next += 1;
        page_ids.push(next);
        next += 1;
    }

    objects.resize(next as usize, Vec::new());

    objects[catalog_id as usize] = format!(
        "{catalog_id} 0 obj\n<< /Type /Catalog /Pages {pages_id} 0 R >>\nendobj\n"
    )
    .into_bytes();

    let kids = page_ids
        .iter()
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    objects[pages_id as usize] = format!(
        "{pages_id} 0 obj\n<< /Type /Pages /Kids [{kids}] /Count {n} >>\nendobj\n"
    )
    .into_bytes();

    for i in 0..n {
        let (jpeg, iw, ih) = &jpegs[i];
        let (pw, ph) = fit_page(*iw, *ih);
        let page_w = pw + 48.0;
        let page_h = ph + 48.0;
        let x = (page_w - pw) / 2.0;
        let y = (page_h - ph) / 2.0;
        let img_id = image_ids[i];
        let content_id = content_ids[i];
        let page_id = page_ids[i];

        let mut img_obj = Vec::new();
        write!(
            &mut img_obj,
            "{img_id} 0 obj\n<< /Type /XObject /Subtype /Image /Width {iw} /Height {ih} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
            jpeg.len()
        )
        .unwrap();
        img_obj.extend_from_slice(jpeg);
        img_obj.extend_from_slice(b"\nendstream\nendobj\n");
        objects[img_id as usize] = img_obj;

        let content = format!("q {pw:.2} 0 0 {ph:.2} {x:.2} {y:.2} cm /Im{i} Do Q\n");
        objects[content_id as usize] = format!(
            "{content_id} 0 obj\n<< /Length {} >>\nstream\n{content}endstream\nendobj\n",
            content.len()
        )
        .into_bytes();

        objects[page_id as usize] = format!(
            "{page_id} 0 obj\n<< /Type /Page /Parent {pages_id} 0 R /MediaBox [0 0 {page_w:.2} {page_h:.2}] /Contents {content_id} 0 R /Resources << /XObject << /Im{i} {img_id} 0 R >> >> >>\nendobj\n"
        )
        .into_bytes();
    }

    assemble_pdf(&objects)
}

pub fn text_to_pdf_bytes(text: &str) -> Result<Vec<u8>> {
    let font_id = 3u32;
    let content_ids_start = 4u32;
    let lines: Vec<&str> = text.lines().collect();
    let lines_per_page = 56usize;
    let pages: Vec<&[&str]> = lines.chunks(lines_per_page).collect();
    let n = pages.len().max(1);
    let pages_id = 2u32;
    let catalog_id = 1u32;
    let mut next = content_ids_start;
    let mut content_ids = Vec::new();
    let mut page_ids = Vec::new();
    for _ in 0..n {
        content_ids.push(next);
        next += 1;
        page_ids.push(next);
        next += 1;
    }
    let mut objects = vec![Vec::new(); next as usize];
    objects[catalog_id as usize] = format!(
        "{catalog_id} 0 obj\n<< /Type /Catalog /Pages {pages_id} 0 R >>\nendobj\n"
    )
    .into_bytes();
    let kids = page_ids
        .iter()
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    objects[pages_id as usize] = format!(
        "{pages_id} 0 obj\n<< /Type /Pages /Kids [{kids}] /Count {n} >>\nendobj\n"
    )
    .into_bytes();
    objects[font_id as usize] =
        b"3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n".to_vec();

    for i in 0..n {
        let chunk = if lines.is_empty() {
            &[""][..]
        } else {
            pages[i]
        };
        let mut stream = String::from("BT /F1 11 Tf 14 TL 56 786 Td\n");
        for line in chunk {
            stream.push_str(&format!("({}) '\n", pdf_escape(line)));
        }
        stream.push_str("ET\n");
        let content_id = content_ids[i];
        let page_id = page_ids[i];
        objects[content_id as usize] = format!(
            "{content_id} 0 obj\n<< /Length {} >>\nstream\n{stream}endstream\nendobj\n",
            stream.len()
        )
        .into_bytes();
        objects[page_id as usize] = format!(
            "{page_id} 0 obj\n<< /Type /Page /Parent {pages_id} 0 R /MediaBox [0 0 595 842] /Contents {content_id} 0 R /Resources << /Font << /F1 {font_id} 0 R >> >> >>\nendobj\n"
        )
        .into_bytes();
    }
    assemble_pdf(&objects)
}

fn pdf_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '(' => "\\(".into(),
            ')' => "\\)".into(),
            '\\' => "\\\\".into(),
            c if c.is_ascii() && !c.is_control() => c.to_string(),
            c => format!("\\{:03o}", c as u32),
        })
        .collect()
}

fn assemble_pdf(objects: &[Vec<u8>]) -> Result<Vec<u8>> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets = vec![0u64; objects.len()];
    for id in 1..objects.len() {
        offsets[id] = pdf.len() as u64;
        pdf.extend_from_slice(&objects[id]);
        if !objects[id].ends_with(b"\n") {
            pdf.push(b'\n');
        }
    }
    let xref_at = pdf.len();
    let count = objects.len();
    pdf.extend_from_slice(format!("xref\n0 {count}\n").as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for id in 1..count {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", offsets[id]).as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {count} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        )
        .as_bytes(),
    );
    Ok(pdf)
}

pub fn pdf_to_txt(src: &Path, settings: &Settings) -> Result<PathBuf> {
    let text = pdf_extract::extract_text(src)
        .map_err(|e| DactError::user(crate::i18n::err_pdf_read(e)))?;
    let out = sibling(src, "", "txt", settings.output)?;
    atomic_write(&out, text.as_bytes())?;
    Ok(out)
}

pub fn merge_pdfs(srcs: &[PathBuf], settings: &Settings) -> Result<PathBuf> {
    if srcs.len() < 2 {
        return Err(DactError::user(crate::i18n::err_merge_two()));
    }
    let mut max_id = 1u32;
    let mut page_ids: Vec<ObjectId> = Vec::new();
    let mut objects: BTreeMap<ObjectId, Object> = BTreeMap::new();

    for src in srcs {
        let mut doc = Document::load(src).map_err(pdf_err)?;
        if doc.is_encrypted() {
            return Err(DactError::PdfLocked);
        }
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;
        for id in doc.get_pages().into_values() {
            page_ids.push(id);
        }
        objects.extend(doc.objects);
    }

    let mut merged = Document::with_version("1.5");
    merged.objects = objects;
    merged.max_id = max_id;

    let pages_id = merged.new_object_id();
    let catalog_id = merged.new_object_id();
    let kids: Vec<Object> = page_ids.iter().map(|id| Object::Reference(*id)).collect();
    let count = page_ids.len() as i64;
    for id in &page_ids {
        if let Ok(Object::Dictionary(dict)) = merged.get_object_mut(*id) {
            dict.set("Parent", Object::Reference(pages_id));
        }
    }
    merged.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => count,
            "Kids" => Object::Array(kids),
        }),
    );
    merged.objects.insert(
        catalog_id,
        Object::Dictionary(dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        }),
    );
    merged.trailer.set("Root", Object::Reference(catalog_id));

    let out = sibling(&srcs[0], crate::i18n::suffix_merged(), "pdf", settings.output)?;
    save_lopdf(&mut merged, &out)?;
    Ok(out)
}

pub fn split_pdf(src: &Path, settings: &Settings) -> Result<Vec<PathBuf>> {
    let doc = Document::load(src).map_err(pdf_err)?;
    if doc.is_encrypted() {
        return Err(DactError::PdfLocked);
    }
    let pages = doc.get_pages();
    if pages.is_empty() {
        return Err(DactError::user(crate::i18n::err_pdf_empty()));
    }
    let mut outs = Vec::new();
    for (i, (_, page_id)) in pages.iter().enumerate() {
        let mut one = Document::with_version("1.5");
        copy_page(&doc, &mut one, *page_id)?;
        let label = if pages.len() == 1 {
            String::new()
        } else {
            crate::i18n::suffix_page(i + 1)
        };
        let out = sibling(src, &label, "pdf", settings.output)?;
        save_lopdf(&mut one, &out)?;
        outs.push(out);
    }
    Ok(outs)
}

pub fn rotate_pdf(src: &Path, settings: &Settings) -> Result<PathBuf> {
    let mut doc = Document::load(src).map_err(pdf_err)?;
    if doc.is_encrypted() {
        return Err(DactError::PdfLocked);
    }
    let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
    for id in pages {
        if let Ok(Object::Dictionary(dict)) = doc.get_object_mut(id) {
            let current = dict
                .get(b"Rotate")
                .ok()
                .and_then(|o| o.as_i64().ok())
                .unwrap_or(0);
            dict.set("Rotate", Object::Integer((current + 90) % 360));
        }
    }
    let out = sibling(src, crate::i18n::suffix_rotated(), "pdf", settings.output)?;
    save_lopdf(&mut doc, &out)?;
    Ok(out)
}

pub fn compress_pdf(src: &Path, settings: &Settings) -> Result<PathBuf> {
    let mut doc = Document::load(src).map_err(pdf_err)?;
    if doc.is_encrypted() {
        return Err(DactError::PdfLocked);
    }
    doc.compress();
    let out = sibling(src, crate::i18n::suffix_compressed(), "pdf", settings.output)?;
    save_lopdf(&mut doc, &out)?;
    Ok(out)
}

pub fn extract_pdf_images(src: &Path, settings: &Settings) -> Result<Vec<PathBuf>> {
    let doc = Document::load(src).map_err(pdf_err)?;
    if doc.is_encrypted() {
        return Err(DactError::PdfLocked);
    }
    let mut outs = Vec::new();
    let mut n = 0u32;
    for object in doc.objects.values() {
        let Object::Stream(stream) = object else {
            continue;
        };
        if !is_image_stream(stream) {
            continue;
        }
        n += 1;
        if let Some((bytes, ext)) = decode_image_stream(stream) {
            let out = sibling(src, &crate::i18n::suffix_frame(n), &ext, settings.output)?;
            atomic_write(&out, &bytes)?;
            outs.push(out);
        }
    }
    if outs.is_empty() {
        return Err(DactError::user(crate::i18n::err_no_embedded()));
    }
    Ok(outs)
}

fn is_image_stream(stream: &Stream) -> bool {
    stream
        .dict
        .get(b"Subtype")
        .ok()
        .and_then(|o| o.as_name().ok())
        .is_some_and(|n| n == b"Image")
}

fn decode_image_stream(stream: &Stream) -> Option<(Vec<u8>, String)> {
    let filter = stream.dict.get(b"Filter").ok();
    let name = match filter {
        Some(Object::Name(n)) => n.as_slice(),
        Some(Object::Array(arr)) => arr.first()?.as_name().ok()?,
        _ => b"",
    };
    match name {
        b"DCTDecode" => Some((stream.content.clone(), "jpg".into())),
        b"JPXDecode" => Some((stream.content.clone(), "jp2".into())),
        b"FlateDecode" => {
            let w = stream.dict.get(b"Width").ok()?.as_i64().ok()? as u32;
            let h = stream.dict.get(b"Height").ok()?.as_i64().ok()? as u32;
            let decoded = stream.decompressed_content().ok()?;
            let img = RgbImage::from_raw(w, h, decoded)?;
            let mut png = Vec::new();
            image::DynamicImage::ImageRgb8(img)
                .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                .ok()?;
            Some((png, "png".into()))
        }
        _ => None,
    }
}

fn copy_page(src: &Document, dst: &mut Document, page_id: ObjectId) -> Result<()> {
    let mut collected: BTreeMap<ObjectId, Object> = BTreeMap::new();
    collect_refs(src, page_id, &mut collected)?;
    let mut remap = BTreeMap::new();
    for old in collected.keys() {
        remap.insert(*old, dst.new_object_id());
    }
    for (old, obj) in collected {
        let new_id = remap[&old];
        dst.objects.insert(new_id, remap_object(obj, &remap));
    }
    let new_page = remap[&page_id];
    let pages_id = dst.new_object_id();
    let catalog_id = dst.new_object_id();
    if let Ok(Object::Dictionary(dict)) = dst.get_object_mut(new_page) {
        dict.set("Parent", Object::Reference(pages_id));
    }
    dst.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => 1,
            "Kids" => vec![Object::Reference(new_page)],
        }),
    );
    dst.objects.insert(
        catalog_id,
        Object::Dictionary(dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        }),
    );
    dst.trailer.set("Root", Object::Reference(catalog_id));
    Ok(())
}

fn collect_refs(
    doc: &Document,
    id: ObjectId,
    out: &mut BTreeMap<ObjectId, Object>,
) -> Result<()> {
    if out.contains_key(&id) {
        return Ok(());
    }
    let obj = doc
        .get_object(id)
        .map_err(|_| DactError::user(crate::i18n::err_pdf_page()))?
        .clone();
    let refs = object_refs(&obj);
    out.insert(id, obj);
    for r in refs {
        collect_refs(doc, r, out)?;
    }
    Ok(())
}

fn object_refs(obj: &Object) -> Vec<ObjectId> {
    match obj {
        Object::Reference(id) => vec![*id],
        Object::Array(arr) => arr.iter().flat_map(object_refs).collect(),
        Object::Dictionary(dict) => dict.iter().flat_map(|(_, v)| object_refs(v)).collect(),
        Object::Stream(stream) => stream
            .dict
            .iter()
            .flat_map(|(_, v)| object_refs(v))
            .collect(),
        _ => vec![],
    }
}

fn remap_object(obj: Object, map: &BTreeMap<ObjectId, ObjectId>) -> Object {
    match obj {
        Object::Reference(id) => Object::Reference(map.get(&id).copied().unwrap_or(id)),
        Object::Array(arr) => {
            Object::Array(arr.into_iter().map(|o| remap_object(o, map)).collect())
        }
        Object::Dictionary(dict) => {
            let mut d = dictionary! {};
            for (k, v) in dict.into_iter() {
                d.set(k, remap_object(v, map));
            }
            Object::Dictionary(d)
        }
        Object::Stream(mut stream) => {
            let mut d = dictionary! {};
            for (k, v) in stream.dict.into_iter() {
                d.set(k, remap_object(v, map));
            }
            stream.dict = d;
            Object::Stream(stream)
        }
        other => other,
    }
}

fn save_lopdf(doc: &mut Document, path: &Path) -> Result<()> {
    let tmp = path.with_file_name(format!(
        ".~{}.part",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("out")
    ));
    let result = (|| {
        doc.save(&tmp)
            .map_err(|e| DactError::user(e.to_string()))?;
        fs::rename(&tmp, path).map_err(DactError::from_io)?;
        Ok(())
    })();
    if tmp.exists() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    #[test]
    fn split_and_merge_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::default();
        let src = dir.path().join("multi.pdf");
        let page = image::RgbImage::from_pixel(32, 24, image::Rgb([10, 20, 30]));
        std::fs::write(&src, images_to_pdf(&[page.clone(), page]).unwrap()).unwrap();
        let parts = split_pdf(&src, &settings).unwrap();
        assert_eq!(parts.len(), 2);
        assert!(src.exists());
        let merged = merge_pdfs(&parts, &settings).unwrap();
        assert!(merged.exists());
        assert_ne!(merged, src);
    }
}

fn pdf_err(err: lopdf::Error) -> DactError {
    let msg = err.to_string();
    if msg.to_ascii_lowercase().contains("encrypt")
        || msg.to_ascii_lowercase().contains("password")
    {
        DactError::PdfLocked
    } else {
        DactError::user(crate::i18n::err_pdf_read(msg))
    }
}
