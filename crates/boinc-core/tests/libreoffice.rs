//! Document conversion tests. The conversion round-trip needs LibreOffice
//! and skips (with a message) when `soffice` is not installed, so it runs on
//! developer machines that have it and quietly no-ops elsewhere.

#![allow(clippy::unwrap_used)]

use std::io::Write;
use std::path::Path;

use boinc_core::converters::libreoffice::find_soffice;
use boinc_core::{ConversionRequest, ConverterRegistry, Format, convert, detect_format};
use zip::write::SimpleFileOptions;

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

const DOCUMENT: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>Hello from the Boinc test suite.</w:t></w:r></w:p></w:body>
</w:document>"#;

fn write_minimal_docx(path: &Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default();
    for (name, body) in [
        ("[Content_Types].xml", CONTENT_TYPES),
        ("_rels/.rels", RELS),
        ("word/document.xml", DOCUMENT),
    ] {
        zip.start_file(name, opts).unwrap();
        zip.write_all(body.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn minimal_docx_is_detected() {
    let dir = tempfile::tempdir().unwrap();
    let docx = dir.path().join("hello.docx");
    write_minimal_docx(&docx);
    assert_eq!(detect_format(&docx).unwrap(), Format::Docx);
}

#[test]
fn docx_pdf_round_trip_via_libreoffice() {
    if find_soffice().is_none() {
        eprintln!("skipping: LibreOffice (soffice) not installed");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let docx = dir.path().join("hello.docx");
    write_minimal_docx(&docx);

    let registry = ConverterRegistry::with_defaults();
    let pdf = convert(
        &registry,
        &ConversionRequest::new(&docx, Format::Pdf),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(detect_format(&pdf.output).unwrap(), Format::Pdf);

    let back = convert(
        &registry,
        &ConversionRequest::new(&pdf.output, Format::Docx),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(detect_format(&back.output).unwrap(), Format::Docx);
}
