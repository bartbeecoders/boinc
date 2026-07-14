//! PDF ↔ Markdown conversion.
//!
//! Markdown → PDF renders the Markdown to HTML (pulldown-cmark) and hands
//! that to headless LibreOffice for layout and PDF export — the same engine
//! and availability rules as the DOCX converters. PDF → Markdown extracts
//! the text layer in-process with `pdf-extract`; layout, images, and styling
//! are not recovered, so the result is flowing paragraphs of plain text —
//! usable, but lossy by nature (scanned PDFs without a text layer come out
//! empty).

use std::path::Path;

use pulldown_cmark::{Options, Parser, html};

use crate::converter::{ConversionOptions, Converter};
use crate::error::ConversionError;
use crate::format::Format;

use super::libreoffice::{absolute, file_url, find_soffice, run_soffice};

pub struct MarkdownToPdf;

impl Converter for MarkdownToPdf {
    fn supports(&self) -> (Format, Format) {
        (Format::Md, Format::Pdf)
    }

    fn is_available(&self) -> bool {
        find_soffice().is_some()
    }

    fn convert(
        &self,
        input: &Path,
        output: &Path,
        _options: &ConversionOptions,
        progress: &mut dyn FnMut(f32),
    ) -> Result<(), ConversionError> {
        progress(0.0);

        // Lossy read: detection only guarantees the sniff window is UTF-8.
        let bytes = std::fs::read(input)?;
        let source = String::from_utf8_lossy(&bytes);

        let parser = Parser::new_ext(
            &source,
            Options::ENABLE_TABLES
                | Options::ENABLE_STRIKETHROUGH
                | Options::ENABLE_FOOTNOTES
                | Options::ENABLE_TASKLISTS,
        );
        let mut body = String::with_capacity(source.len() * 2);
        html::push_html(&mut body, parser);

        // <base> makes relative image links resolve against the Markdown
        // file's directory rather than the temp dir the HTML lives in.
        let input = absolute(input)?;
        let base = input
            .parent()
            .map(|dir| format!(r#"<base href="{}/">"#, file_url(dir)))
            .unwrap_or_default();
        let doc = format!(
            "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\">{base}</head><body>\n{body}</body></html>\n"
        );

        let work = tempfile::tempdir()?;
        let html_path = work.path().join("input.html");
        std::fs::write(&html_path, doc)?;
        progress(0.2);

        run_soffice(&html_path, output, "pdf", None, "pdf")?;
        progress(1.0);
        Ok(())
    }
}

pub struct PdfToMarkdown;

impl Converter for PdfToMarkdown {
    fn supports(&self) -> (Format, Format) {
        (Format::Pdf, Format::Md)
    }

    fn convert(
        &self,
        input: &Path,
        output: &Path,
        _options: &ConversionOptions,
        progress: &mut dyn FnMut(f32),
    ) -> Result<(), ConversionError> {
        progress(0.0);

        // pdf-extract is known to panic on malformed PDFs; contain that so a
        // bad file fails this one job instead of the whole worker thread.
        let text = match std::panic::catch_unwind(|| pdf_extract::extract_text(input)) {
            Ok(Ok(text)) => text,
            Ok(Err(e)) => {
                return Err(ConversionError::ToolFailed {
                    tool: "PDF text extraction".into(),
                    message: e.to_string(),
                });
            }
            Err(_) => {
                return Err(ConversionError::ToolFailed {
                    tool: "PDF text extraction".into(),
                    message: "the PDF parser gave up on this file".into(),
                });
            }
        };
        progress(0.9);

        std::fs::write(output, normalize(&text))?;
        progress(1.0);
        Ok(())
    }
}

/// Tidy extracted text into presentable Markdown: strip trailing whitespace,
/// and collapse the blank-line runs and page breaks PDF extraction produces
/// into single blank lines (paragraph breaks).
fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_break = false;
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            pending_break = true;
            continue;
        }
        if !out.is_empty() {
            out.push_str(if pending_break { "\n\n" } else { "\n" });
        }
        out.push_str(line);
        pending_break = false;
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_collapses_gaps_and_trailing_whitespace() {
        assert_eq!(
            normalize("Title  \n\n\n\nBody line one\nline two\n\x0C\nNext page\n"),
            "Title\n\nBody line one\nline two\n\nNext page\n"
        );
        assert_eq!(normalize(""), "");
        assert_eq!(normalize("\n\n\n"), "");
    }
}
