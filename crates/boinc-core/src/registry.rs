use std::collections::HashMap;
use std::sync::Arc;

use crate::converter::Converter;
use crate::converters;
use crate::format::Format;

/// Registry of converters keyed by (input, output) format pair. Everything
/// downstream — context-menu entries, CLI listings, the app's format picker —
/// is generated from this.
#[derive(Default)]
pub struct ConverterRegistry {
    converters: HashMap<(Format, Format), Arc<dyn Converter>>,
}

impl ConverterRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// A registry with all built-in converters. This is the single place a
    /// new built-in converter must be added (see this crate's README).
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Every raster pair + every raster → SVG. Driven by Format::RASTERS so
        // adding a bitmap format is one enum entry plus image-crate features.
        for &from in &Format::RASTERS {
            registry.register(Arc::new(converters::vector::BitmapToSvg::new(from)));
            for &to in &Format::RASTERS {
                if from != to {
                    registry.register(Arc::new(converters::raster::RasterConverter::new(from, to)));
                }
            }
        }

        registry.register(Arc::new(
            converters::libreoffice::LibreOfficeConverter::docx_to_pdf(),
        ));
        registry.register(Arc::new(
            converters::libreoffice::LibreOfficeConverter::pdf_to_docx(),
        ));
        registry.register(Arc::new(converters::markdown::MarkdownToPdf));
        registry.register(Arc::new(converters::markdown::PdfToMarkdown));
        registry
    }

    /// Register a converter, replacing any existing one for the same pair.
    pub fn register(&mut self, converter: Arc<dyn Converter>) {
        self.converters.insert(converter.supports(), converter);
    }

    /// The converter for a format pair, if registered.
    pub fn get(&self, from: Format, to: Format) -> Option<&Arc<dyn Converter>> {
        self.converters.get(&(from, to))
    }

    /// Target formats reachable from `from` with converters that are
    /// available on this machine, sorted for stable menu order.
    pub fn available_targets(&self, from: Format) -> Vec<Format> {
        let mut targets: Vec<Format> = self
            .converters
            .iter()
            .filter(|((f, _), converter)| *f == from && converter.is_available())
            .map(|((_, to), _)| *to)
            .collect();
        targets.sort();
        targets
    }

    /// All registered (from, to) pairs, including currently unavailable ones,
    /// sorted.
    pub fn pairs(&self) -> Vec<(Format, Format)> {
        let mut pairs: Vec<_> = self.converters.keys().copied().collect();
        pairs.sort();
        pairs
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::converter::ConversionOptions;
    use crate::error::ConversionError;

    struct Dummy {
        pair: (Format, Format),
        available: bool,
    }

    impl Converter for Dummy {
        fn supports(&self) -> (Format, Format) {
            self.pair
        }

        fn is_available(&self) -> bool {
            self.available
        }

        fn convert(
            &self,
            _input: &Path,
            _output: &Path,
            _options: &ConversionOptions,
            _progress: &mut dyn FnMut(f32),
        ) -> Result<(), ConversionError> {
            Ok(())
        }
    }

    #[test]
    fn lookup_and_listing() {
        let mut registry = ConverterRegistry::new();
        registry.register(Arc::new(Dummy {
            pair: (Format::Png, Format::Jpg),
            available: true,
        }));
        registry.register(Arc::new(Dummy {
            pair: (Format::Png, Format::Pdf),
            available: false,
        }));

        assert!(registry.get(Format::Png, Format::Jpg).is_some());
        assert!(registry.get(Format::Jpg, Format::Png).is_none());

        // Unavailable converters are hidden from target listings but still
        // count as registered pairs.
        assert_eq!(registry.available_targets(Format::Png), vec![Format::Jpg]);
        assert_eq!(
            registry.pairs(),
            vec![(Format::Png, Format::Jpg), (Format::Png, Format::Pdf)]
        );
    }

    #[test]
    fn defaults_cover_planned_pairs() {
        let registry = ConverterRegistry::with_defaults();
        for &from in &Format::RASTERS {
            assert!(
                registry.get(from, Format::Svg).is_some(),
                "{from} -> SVG missing"
            );
            for &to in &Format::RASTERS {
                if from != to {
                    assert!(
                        registry.get(from, to).is_some(),
                        "{from} -> {to} missing"
                    );
                }
            }
        }
        for (from, to) in [
            (Format::Docx, Format::Pdf),
            (Format::Pdf, Format::Docx),
            (Format::Md, Format::Pdf),
            (Format::Pdf, Format::Md),
        ] {
            assert!(registry.get(from, to).is_some(), "{from} -> {to} missing");
        }
    }
}
