//! Context-menu entries derived from the converter registry, plus the
//! per-platform type identifiers each entry needs.

use boinc_core::{ConverterRegistry, Format};

/// One "Convert to X" context-menu entry for files of format `from`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuEntry {
    pub from: Format,
    pub to: Format,
}

impl MenuEntry {
    /// Menu label, e.g. "Convert to JPG".
    pub fn label(&self) -> String {
        format!("Convert to {}", self.to)
    }

    /// Stable identifier used in file names, registry keys, and action ids.
    pub fn id(&self) -> String {
        format!("{}-to-{}", self.from.extension(), self.to.extension())
    }
}

/// Entries for every conversion that is available on this machine right now
/// (tool-backed converters whose tool is missing are skipped — re-run
/// `boinc integrate install` after installing the tool). Sorted for stable
/// output.
pub fn menu_entries(registry: &ConverterRegistry) -> Vec<MenuEntry> {
    let mut entries: Vec<MenuEntry> = registry
        .pairs()
        .into_iter()
        .filter(|(from, to)| {
            registry
                .get(*from, *to)
                .is_some_and(|converter| converter.is_available())
        })
        .map(|(from, to)| MenuEntry { from, to })
        .collect();
    entries.sort_by_key(|e| (e.from, e.to));
    entries
}

/// XDG MIME type (Linux service menus).
pub fn mime_type(format: Format) -> &'static str {
    match format {
        Format::Png => "image/png",
        Format::Jpg => "image/jpeg",
        Format::Pdf => "application/pdf",
        Format::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    }
}

/// File extensions (with dot) that select this format on Windows.
pub fn windows_extensions(format: Format) -> &'static [&'static str] {
    match format {
        Format::Png => &[".png"],
        Format::Jpg => &[".jpg", ".jpeg"],
        Format::Pdf => &[".pdf"],
        Format::Docx => &[".docx"],
    }
}

/// Uniform Type Identifier (macOS Quick Actions).
pub fn uti(format: Format) -> &'static str {
    match format {
        Format::Png => "public.png",
        Format::Jpg => "public.jpeg",
        Format::Pdf => "com.adobe.pdf",
        Format::Docx => "org.openxmlformats.wordprocessingml.document",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_are_sorted_and_labeled() {
        let registry = ConverterRegistry::with_defaults();
        let entries = menu_entries(&registry);
        // PNG<->JPG are always available; document pairs depend on LibreOffice.
        assert!(entries.contains(&MenuEntry {
            from: Format::Png,
            to: Format::Jpg
        }));
        let png_jpg = MenuEntry {
            from: Format::Png,
            to: Format::Jpg,
        };
        assert_eq!(png_jpg.label(), "Convert to JPG");
        assert_eq!(png_jpg.id(), "png-to-jpg");
        let mut sorted = entries.clone();
        sorted.sort_by_key(|e| (e.from, e.to));
        assert_eq!(entries, sorted);
    }
}
