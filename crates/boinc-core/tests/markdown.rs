//! Markdown conversion tests. PDF → Markdown is pure Rust and runs
//! everywhere; Markdown → PDF delegates to LibreOffice and skips (with a
//! message) when `soffice` is not installed, like the DOCX tests.

#![allow(clippy::unwrap_used)]

use std::path::Path;

use boinc_core::converters::libreoffice::find_soffice;
use boinc_core::{
    ConversionError, ConversionRequest, ConverterRegistry, Format, convert, detect_format,
};
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, Stream, dictionary};

/// A one-page PDF with a single line of Helvetica text.
fn write_test_pdf(path: &Path, text: &str) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id },
    });
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 24.into()]),
            Operation::new("Td", vec![72.into(), 720.into()]),
            Operation::new("Tj", vec![Object::string_literal(text)]),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => resources_id,
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.save(path).unwrap();
}

#[test]
fn markdown_is_detected() {
    let dir = tempfile::tempdir().unwrap();
    let md = dir.path().join("notes.md");
    std::fs::write(&md, "# Heading\n\nSome *text* with `code`.\n").unwrap();
    assert_eq!(detect_format(&md).unwrap(), Format::Md);

    let markdown = dir.path().join("notes.markdown");
    std::fs::write(&markdown, "plain paragraph\n").unwrap();
    assert_eq!(detect_format(&markdown).unwrap(), Format::Md);
}

#[test]
fn binary_data_with_md_extension_is_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let fake = dir.path().join("fake.md");
    std::fs::write(&fake, [0u8, 159, 146, 150, 7, 3]).unwrap();
    assert!(matches!(
        detect_format(&fake).unwrap_err(),
        ConversionError::UnknownFormat { .. }
    ));
}

#[test]
fn pdf_to_markdown_extracts_text() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("doc.pdf");
    write_test_pdf(&pdf, "Hello from the Boinc test suite");
    assert_eq!(detect_format(&pdf).unwrap(), Format::Pdf);

    let registry = ConverterRegistry::with_defaults();
    let result = convert(
        &registry,
        &ConversionRequest::new(&pdf, Format::Md),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(result.output, dir.path().join("doc.md"));
    assert_eq!(detect_format(&result.output).unwrap(), Format::Md);

    let text = std::fs::read_to_string(&result.output).unwrap();
    assert!(
        text.contains("Hello from the Boinc test suite"),
        "extracted text was: {text:?}"
    );
}

#[test]
fn md_pdf_round_trip_via_libreoffice() {
    if find_soffice().is_none() {
        eprintln!("skipping: LibreOffice (soffice) not installed");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let md = dir.path().join("hello.md");
    std::fs::write(
        &md,
        "# Boincmark heading\n\nA paragraph with **bold** text.\n\n- first item\n- second item\n",
    )
    .unwrap();

    let registry = ConverterRegistry::with_defaults();
    let pdf = convert(
        &registry,
        &ConversionRequest::new(&md, Format::Pdf),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(detect_format(&pdf.output).unwrap(), Format::Pdf);

    let back = convert(
        &registry,
        &ConversionRequest::new(&pdf.output, Format::Md),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(detect_format(&back.output).unwrap(), Format::Md);
    let text = std::fs::read_to_string(&back.output).unwrap();
    assert!(
        text.contains("Boincmark heading"),
        "round-tripped text was: {text:?}"
    );
}
