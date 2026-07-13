use std::path::Path;

use crate::error::ConversionError;
use crate::format::Format;

/// Options shared by all conversions. Converters read the fields relevant to
/// them and ignore the rest.
#[derive(Debug, Clone)]
pub struct ConversionOptions {
    /// JPEG encoding quality, 1–100.
    pub jpeg_quality: u8,
    /// Background color (RGB) used when flattening transparency away.
    pub background: [u8; 3],
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            jpeg_quality: 90,
            background: [255, 255, 255],
        }
    }
}

/// A single (input format → output format) conversion implementation.
///
/// Implementations must be thread-safe: the app runs conversions on worker
/// threads.
pub trait Converter: Send + Sync {
    /// The (from, to) pair this converter handles.
    fn supports(&self) -> (Format, Format);

    /// Whether the converter can run on this machine (e.g. an external tool
    /// it delegates to is installed). Unavailable converters are hidden from
    /// context menus and CLI listings.
    fn is_available(&self) -> bool {
        true
    }

    /// Convert `input` (already verified to be the supported input format)
    /// into `output`. `progress` is called with fractions in `0.0..=1.0`.
    fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
        progress: &mut dyn FnMut(f32),
    ) -> Result<(), ConversionError>;
}
