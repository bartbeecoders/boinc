use std::fmt;
use std::io::Read;
use std::path::Path;

use crate::error::ConversionError;

/// A file format Boinc can convert between.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Format {
    Png,
    Jpg,
    Pdf,
    Docx,
    Md,
}

impl Format {
    pub const ALL: [Format; 5] = [
        Format::Png,
        Format::Jpg,
        Format::Pdf,
        Format::Docx,
        Format::Md,
    ];

    /// Canonical file extension, lowercase, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpg => "jpg",
            Format::Pdf => "pdf",
            Format::Docx => "docx",
            Format::Md => "md",
        }
    }

    /// Name shown in menus and messages.
    pub fn display_name(self) -> &'static str {
        match self {
            Format::Png => "PNG",
            Format::Jpg => "JPG",
            Format::Pdf => "PDF",
            Format::Docx => "DOCX",
            Format::Md => "Markdown",
        }
    }

    /// Parse an extension (with or without leading dot, any case), accepting
    /// aliases such as `jpeg`.
    pub fn from_extension(ext: &str) -> Option<Format> {
        match ext.trim_start_matches('.').to_ascii_lowercase().as_str() {
            "png" => Some(Format::Png),
            "jpg" | "jpeg" => Some(Format::Jpg),
            "pdf" => Some(Format::Pdf),
            "docx" => Some(Format::Docx),
            "md" | "markdown" => Some(Format::Md),
            _ => None,
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

/// How many leading bytes `detect_format` reads to identify a file. Large
/// enough that a DOCX's early ZIP entry names ("word/document.xml") fall
/// inside the window.
const SNIFF_LEN: usize = 64 * 1024;

/// What the leading bytes of a file tell us on their own.
enum Sniffed {
    Known(Format),
    /// A ZIP container that does not look like DOCX; the extension decides.
    Zip,
    /// Plain text — Markdown has no magic bytes; the extension decides.
    Text,
    Unknown,
}

fn sniff(bytes: &[u8]) -> Sniffed {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Sniffed::Known(Format::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Sniffed::Known(Format::Jpg)
    } else if bytes.starts_with(b"%PDF-") {
        Sniffed::Known(Format::Pdf)
    } else if bytes.starts_with(b"PK\x03\x04") {
        if contains(bytes, b"word/") {
            Sniffed::Known(Format::Docx)
        } else {
            Sniffed::Zip
        }
    } else if is_plain_text(bytes) {
        Sniffed::Text
    } else {
        Sniffed::Unknown
    }
}

/// Printable text: valid UTF-8 with no control bytes beyond whitespace. A
/// multi-byte sequence cut off at the sniff-window boundary still counts.
fn is_plain_text(bytes: &[u8]) -> bool {
    match std::str::from_utf8(bytes) {
        Ok(_) => {}
        // error_len() == None means the only defect is a trailing sequence
        // truncated by the read window, not invalid data.
        Err(e) if e.error_len().is_none() => {}
        Err(_) => return false,
    }
    !bytes
        .iter()
        .any(|&b| b < 0x20 && !matches!(b, b'\t' | b'\n' | b'\r' | 0x0C))
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Determine a file's format from its content. The extension is only
/// consulted where content sniffing is ambiguous (ZIP containers whose entry
/// listing didn't identify them); it is never trusted against conflicting
/// content.
pub fn detect_format(path: &Path) -> Result<Format, ConversionError> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = vec![0u8; SNIFF_LEN];
    let mut filled = 0;
    loop {
        let n = file.read(&mut buf[filled..])?;
        if n == 0 {
            break;
        }
        filled += n;
        if filled == buf.len() {
            break;
        }
    }
    buf.truncate(filled);

    let by_extension = path
        .extension()
        .and_then(|e| e.to_str())
        .and_then(Format::from_extension);

    match sniff(&buf) {
        Sniffed::Known(format) => Ok(format),
        Sniffed::Zip if by_extension == Some(Format::Docx) => Ok(Format::Docx),
        Sniffed::Text if by_extension == Some(Format::Md) => Ok(Format::Md),
        _ => Err(ConversionError::UnknownFormat {
            path: path.to_path_buf(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_parsing() {
        assert_eq!(Format::from_extension("png"), Some(Format::Png));
        assert_eq!(Format::from_extension(".PNG"), Some(Format::Png));
        assert_eq!(Format::from_extension("jpeg"), Some(Format::Jpg));
        assert_eq!(Format::from_extension("JPG"), Some(Format::Jpg));
        assert_eq!(Format::from_extension("md"), Some(Format::Md));
        assert_eq!(Format::from_extension("markdown"), Some(Format::Md));
        assert_eq!(Format::from_extension("txt"), None);
    }

    #[test]
    fn sniffs_magic_bytes() {
        assert!(matches!(
            sniff(b"\x89PNG\r\n\x1a\n...."),
            Sniffed::Known(Format::Png)
        ));
        assert!(matches!(
            sniff(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Sniffed::Known(Format::Jpg)
        ));
        assert!(matches!(sniff(b"%PDF-1.7\n"), Sniffed::Known(Format::Pdf)));
        assert!(matches!(
            sniff(b"PK\x03\x04....word/document.xml"),
            Sniffed::Known(Format::Docx)
        ));
        assert!(matches!(sniff(b"PK\x03\x04....other.txt"), Sniffed::Zip));
        assert!(matches!(
            sniff(b"# Heading\n\nBody *text*.\n"),
            Sniffed::Text
        ));
        assert!(matches!(sniff("héllo".as_bytes()), Sniffed::Text));
        // A multi-byte character split by the sniff window is still text...
        assert!(matches!(sniff(&"héllo".as_bytes()[..2]), Sniffed::Text));
        // ...but invalid UTF-8 or control bytes in the middle are not.
        assert!(matches!(sniff(&[0x68, 0xFF, 0x69]), Sniffed::Unknown));
        assert!(matches!(sniff(b"hel\x00lo"), Sniffed::Unknown));
    }
}
