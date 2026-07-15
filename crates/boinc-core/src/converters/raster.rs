//! Raster image conversion built on the `image` crate.
//!
//! One [`RasterConverter`] instance per (from, to) pair among
//! [`Format::RASTERS`]. JPEG encoding flattens alpha onto the configured
//! background; other formats preserve alpha when the container supports it.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat, RgbImage, RgbaImage};

use crate::converter::{ConversionOptions, Converter};
use crate::error::ConversionError;
use crate::format::Format;

/// Raster image conversion; one instance per (from, to) pair.
pub struct RasterConverter {
    from: Format,
    to: Format,
}

impl RasterConverter {
    /// Build a converter for a raster pair. Both ends must be rasters and
    /// distinct; callers (the registry) are responsible for that.
    pub fn new(from: Format, to: Format) -> Self {
        debug_assert!(from.is_raster() && to.is_raster() && from != to);
        Self { from, to }
    }

    pub fn png_to_jpg() -> Self {
        Self::new(Format::Png, Format::Jpg)
    }

    pub fn jpg_to_png() -> Self {
        Self::new(Format::Jpg, Format::Png)
    }
}

impl Converter for RasterConverter {
    fn supports(&self) -> (Format, Format) {
        (self.from, self.to)
    }

    fn convert(
        &self,
        input: &Path,
        output: &Path,
        options: &ConversionOptions,
        progress: &mut dyn FnMut(f32),
    ) -> Result<(), ConversionError> {
        progress(0.0);
        let img = image::open(input)?;
        progress(0.5);
        save_raster(img, self.from, self.to, output, options)?;
        progress(1.0);
        Ok(())
    }
}

fn save_raster(
    img: DynamicImage,
    from: Format,
    to: Format,
    output: &Path,
    options: &ConversionOptions,
) -> Result<(), ConversionError> {
    match to {
        Format::Jpg => {
            // JPEG has no alpha channel: flatten onto the configured
            // background color first.
            let flattened = flatten(img.to_rgba8(), options.background);
            let writer = BufWriter::new(File::create(output)?);
            let encoder =
                JpegEncoder::new_with_quality(writer, options.jpeg_quality.clamp(1, 100));
            flattened.write_with_encoder(encoder)?;
        }
        Format::Png => {
            img.save_with_format(output, ImageFormat::Png)?;
        }
        Format::Bmp => {
            img.save_with_format(output, ImageFormat::Bmp)?;
        }
        Format::Gif => {
            img.save_with_format(output, ImageFormat::Gif)?;
        }
        Format::WebP => {
            img.save_with_format(output, ImageFormat::WebP)?;
        }
        other => {
            return Err(ConversionError::Unsupported { from, to: other });
        }
    }
    Ok(())
}

/// Alpha-blend every pixel over `background`.
fn flatten(rgba: RgbaImage, background: [u8; 3]) -> RgbImage {
    let (width, height) = rgba.dimensions();
    let mut out = RgbImage::new(width, height);
    for (x, y, pixel) in rgba.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        let alpha = u16::from(a);
        let blend = |fg: u8, bg: u8| -> u8 {
            ((u16::from(fg) * alpha + u16::from(bg) * (255 - alpha)) / 255) as u8
        };
        out.put_pixel(
            x,
            y,
            image::Rgb([
                blend(r, background[0]),
                blend(g, background[1]),
                blend(b, background[2]),
            ]),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_blends_transparency_onto_background() {
        let mut img = RgbaImage::new(2, 1);
        img.put_pixel(0, 0, image::Rgba([10, 20, 30, 255])); // opaque
        img.put_pixel(1, 0, image::Rgba([10, 20, 30, 0])); // fully transparent

        let out = flatten(img, [200, 100, 50]);
        assert_eq!(out.get_pixel(0, 0).0, [10, 20, 30]);
        assert_eq!(out.get_pixel(1, 0).0, [200, 100, 50]);
    }
}
