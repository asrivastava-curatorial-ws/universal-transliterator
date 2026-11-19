#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use tempfile::NamedTempFile;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

pub const MAX_EXPORT_BYTES: usize = 16 * 1024 * 1024;

pub fn write_utf8(path: &Path, text: &str) -> io::Result<()> {
    if text.len() > MAX_EXPORT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "document is too large to export",
        ));
    }
    let parent = output_parent(path, false)?;
    let mut file = NamedTempFile::new_in(parent)?;
    file.write_all(text.as_bytes())?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path).map_err(|error| error.error)?;
    File::open(parent)?.sync_all()
}

/// Layout hints for DOCX export.
#[derive(Clone, Debug, Default)]
pub struct DocxStyle {
    /// Right-to-left paragraphs (Arabic, Persian, Hebrew, Urdu…).
    pub rtl: bool,
    /// Font for complex scripts (Indic, Arabic…); Word substitutes a local
    /// font if it is not installed.
    pub complex_font: Option<String>,
}

/// Writes `text` as a minimal Word document. With `overwrite`, an existing
/// file is replaced atomically (the save dialog already asked).
pub fn write_docx(path: &Path, text: &str, overwrite: bool) -> io::Result<()> {
    write_docx_styled(path, text, overwrite, &DocxStyle::default())
}

pub fn write_docx_styled(
    path: &Path,
    text: &str,
    overwrite: bool,
    style: &DocxStyle,
) -> io::Result<()> {
    if text.len() > MAX_EXPORT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "document is too large to export",
        ));
    }
    let parent = output_parent(path, overwrite)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    let mut zip = ZipWriter::new(temporary.as_file_mut());
    let stored = FileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated = FileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("[Content_Types].xml", stored)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"#)?;
    zip.start_file("_rels/.rels", deflated)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#)?;
    zip.start_file("word/_rels/document.xml.rels", deflated)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#)?;
    zip.start_file("word/styles.xml", deflated)?;
    let complex = xml_escape(style.complex_font.as_deref().unwrap_or("Noto Sans"));
    zip.write_all(format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:rPr><w:rFonts w:ascii="Noto Sans" w:hAnsi="Noto Sans" w:eastAsia="Noto Sans CJK SC" w:cs="{complex}"/><w:sz w:val="24"/><w:szCs w:val="24"/><w:lang w:val="en-US" w:eastAsia="zh-CN" w:bidi="ar-SA"/></w:rPr></w:style></w:styles>"#).as_bytes())?;
    zip.start_file("word/document.xml", deflated)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#)?;
    let (paragraph, run) = if style.rtl {
        (
            r#"<w:p><w:pPr><w:bidi/></w:pPr><w:r><w:rPr><w:rtl/></w:rPr><w:t xml:space="preserve">"#,
            "",
        )
    } else {
        (r#"<w:p><w:r><w:t xml:space="preserve">"#, "")
    };
    for line in text.split('\n') {
        zip.write_all(paragraph.as_bytes())?;
        zip.write_all(run.as_bytes())?;
        zip.write_all(xml_escape(line.trim_end_matches('\r')).as_bytes())?;
        zip.write_all(br#"</w:t></w:r></w:p>"#)?;
    }
    zip.write_all(br#"<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr></w:body></w:document>"#)?;
    zip.finish()?.sync_all()?;
    drop(zip);
    if overwrite {
        temporary.persist(path).map_err(|error| error.error)?;
    } else {
        temporary
            .persist_noclobber(path)
            .map_err(|error| error.error)?;
    }
    File::open(parent)?.sync_all()
}

fn output_parent(path: &Path, overwrite: bool) -> io::Result<&Path> {
    if path
        .symlink_metadata()
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "refusing to replace a symbolic link",
        ));
    }
    if path.exists() && !overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "destination already exists",
        ));
    }
    path.parent()
        .filter(|parent| parent.is_dir())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid output directory"))
}

fn xml_escape(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            ch if is_valid_xml_char(ch) => output.push(ch),
            _ => output.push('\u{fffd}'),
        }
    }
    output
}

fn is_valid_xml_char(ch: char) -> bool {
    matches!(ch, '\u{9}' | '\u{a}' | '\u{d}')
        || ('\u{20}'..='\u{d7ff}').contains(&ch)
        || ('\u{e000}'..='\u{fffd}').contains(&ch)
        || ('\u{10000}'..='\u{10ffff}').contains(&ch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn docx_is_valid_zip_and_escapes_markup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.docx");
        write_docx(&path, "नमस्ते & <你好>\nمرحبا", false).unwrap();
        let mut archive = zip::ZipArchive::new(File::open(path).unwrap()).unwrap();
        let mut xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        assert!(xml.contains("नमस्ते &amp; &lt;你好&gt;"));
        assert!(!xml.contains("<你好>"));
    }

    #[test]
    fn docx_overwrite_is_explicit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.docx");
        write_docx(&path, "one", false).unwrap();
        assert_eq!(
            write_docx(&path, "two", false).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        let style = DocxStyle {
            rtl: true,
            complex_font: Some("Noto Naskh Arabic".into()),
        };
        write_docx_styled(&path, "مرحبا", true, &style).unwrap();
        let mut archive = zip::ZipArchive::new(File::open(&path).unwrap()).unwrap();
        let mut xml = String::new();
        archive
            .by_name("word/document.xml")
            .unwrap()
            .read_to_string(&mut xml)
            .unwrap();
        assert!(xml.contains("<w:bidi/>") && xml.contains("مرحبا"));
    }

    #[test]
    fn exports_refuse_to_replace_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("existing.txt");
        std::fs::write(&path, "keep me").unwrap();
        assert_eq!(
            write_utf8(&path, "replacement").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read_to_string(path).unwrap(), "keep me");
    }
}
