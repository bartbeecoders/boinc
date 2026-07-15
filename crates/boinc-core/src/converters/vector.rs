//! Bitmap → SVG conversion via [vtracer](https://github.com/visioncortex/vtracer).
//!
//! Decoding uses our `image` crate (PNG/JPG/BMP/GIF/WebP) so we are not limited
//! to the older `image` version vtracer pins for its own `convert_image_to_svg`.

use std::path::Path;

use crate::converter::{ConversionOptions, Converter};
use crate::error::ConversionError;
use crate::format::Format;

/// Raster image to SVG; one instance per input bitmap format.
pub struct BitmapToSvg {
    from: Format,
}

impl BitmapToSvg {
    /// Build a converter for any raster input format.
    pub fn new(from: Format) -> Self {
        debug_assert!(from.is_raster());
        Self { from }
    }

    pub fn png() -> Self {
        Self::new(Format::Png)
    }

    pub fn jpg() -> Self {
        Self::new(Format::Jpg)
    }
}

impl Converter for BitmapToSvg {
    fn supports(&self) -> (Format, Format) {
        (self.from, Format::Svg)
    }

    fn convert(
        &self,
        input: &Path,
        output: &Path,
        _options: &ConversionOptions,
        progress: &mut dyn FnMut(f32),
    ) -> Result<(), ConversionError> {
        progress(0.0);

        let rgba = image::open(input)?.to_rgba8();
        progress(0.3);
        let (width, height) = rgba.dimensions();
        let color_img = vtracer::ColorImage {
            pixels: rgba.into_raw(),
            width: width as usize,
            height: height as usize,
        };

        // Default config is the color-mode stack used by the vtracer CLI
        // without a preset; good general-purpose settings for photos and
        // graphics alike.
        let svg = vtracer::convert(color_img, vtracer::Config::default()).map_err(|message| {
            ConversionError::ToolFailed {
                tool: "vtracer".into(),
                message,
            }
        })?;
        progress(0.9);

        std::fs::write(output, svg.to_string())?;
        progress(1.0);
        Ok(())
    }
}
