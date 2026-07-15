//! End-to-end tests of the conversion pipeline through the public API.

#![allow(clippy::unwrap_used)]

use std::path::Path;

use boinc_core::{
    ConversionError, ConversionRequest, ConverterRegistry, Format, convert, detect_format,
};

fn registry() -> ConverterRegistry {
    ConverterRegistry::with_defaults()
}

fn write_test_png(path: &Path) {
    let mut img = image::RgbaImage::new(3, 2);
    for (i, pixel) in img.pixels_mut().enumerate() {
        *pixel = image::Rgba([(i * 40) as u8, 100, 200, 255]);
    }
    img.save(path).unwrap();
}

#[test]
fn png_jpg_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);

    let mut updates = Vec::new();
    let result = convert(
        &registry(),
        &ConversionRequest::new(&png, Format::Jpg),
        &mut |p| updates.push(p),
    )
    .unwrap();
    assert_eq!(result.output, dir.path().join("pic.jpg"));
    assert_eq!(detect_format(&result.output).unwrap(), Format::Jpg);
    assert_eq!(updates.first().copied(), Some(0.0));
    assert_eq!(updates.last().copied(), Some(1.0));

    let back = convert(
        &registry(),
        &ConversionRequest::new(&result.output, Format::Png),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(detect_format(&back.output).unwrap(), Format::Png);
    let img = image::open(&back.output).unwrap();
    assert_eq!((img.width(), img.height()), (3, 2));
}

#[test]
fn transparent_png_flattens_onto_background() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("transparent.png");
    let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 0]));
    img.save(&png).unwrap();

    let mut request = ConversionRequest::new(&png, Format::Jpg);
    request.options.background = [255, 0, 0];
    let result = convert(&registry(), &request, &mut |_| {}).unwrap();

    let jpg = image::open(&result.output).unwrap().to_rgb8();
    let pixel = jpg.get_pixel(4, 4).0;
    // JPEG is lossy; a fully transparent input must come back approximately
    // the configured background color.
    assert!(
        pixel[0] > 220 && pixel[1] < 40 && pixel[2] < 40,
        "expected ~red background, got {pixel:?}"
    );
}

#[test]
fn default_output_never_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);

    let first = convert(
        &registry(),
        &ConversionRequest::new(&png, Format::Jpg),
        &mut |_| {},
    )
    .unwrap();
    let second = convert(
        &registry(),
        &ConversionRequest::new(&png, Format::Jpg),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(first.output, dir.path().join("pic.jpg"));
    assert_eq!(second.output, dir.path().join("pic (1).jpg"));
}

#[test]
fn explicit_existing_output_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);
    let taken = dir.path().join("out.jpg");
    std::fs::write(&taken, b"precious").unwrap();

    let mut request = ConversionRequest::new(&png, Format::Jpg);
    request.output = Some(taken.clone());
    let err = convert(&registry(), &request, &mut |_| {}).unwrap_err();
    assert!(matches!(err, ConversionError::OutputExists { .. }));
    assert_eq!(std::fs::read(&taken).unwrap(), b"precious");
}

#[test]
fn unsupported_pair_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);

    let err = convert(
        &registry(),
        &ConversionRequest::new(&png, Format::Docx),
        &mut |_| {},
    )
    .unwrap_err();
    assert!(matches!(err, ConversionError::Unsupported { .. }));
}

#[test]
fn detection_ignores_wrong_extension() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("real.png");
    write_test_png(&png);
    let mislabeled = dir.path().join("actually-a-png.jpg");
    std::fs::rename(&png, &mislabeled).unwrap();

    assert_eq!(detect_format(&mislabeled).unwrap(), Format::Png);
}

#[test]
fn garbage_input_is_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let junk = dir.path().join("junk.png");
    std::fs::write(&junk, b"not an image at all").unwrap();

    let err = convert(
        &registry(),
        &ConversionRequest::new(&junk, Format::Jpg),
        &mut |_| {},
    )
    .unwrap_err();
    assert!(matches!(err, ConversionError::UnknownFormat { .. }));
}

#[test]
fn png_to_svg() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);

    let mut updates = Vec::new();
    let result = convert(
        &registry(),
        &ConversionRequest::new(&png, Format::Svg),
        &mut |p| updates.push(p),
    )
    .unwrap();
    assert_eq!(result.output, dir.path().join("pic.svg"));
    assert_eq!(detect_format(&result.output).unwrap(), Format::Svg);
    assert_eq!(updates.first().copied(), Some(0.0));
    assert_eq!(updates.last().copied(), Some(1.0));

    let body = std::fs::read_to_string(&result.output).unwrap();
    assert!(
        body.to_ascii_lowercase().contains("<svg"),
        "expected SVG markup, got: {body}"
    );
}

#[test]
fn jpg_to_svg() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);
    let jpg = convert(
        &registry(),
        &ConversionRequest::new(&png, Format::Jpg),
        &mut |_| {},
    )
    .unwrap()
    .output;

    let result = convert(
        &registry(),
        &ConversionRequest::new(&jpg, Format::Svg),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(detect_format(&result.output).unwrap(), Format::Svg);
    let body = std::fs::read_to_string(&result.output).unwrap();
    assert!(body.to_ascii_lowercase().contains("<svg"));
}

#[test]
fn bmp_gif_webp_round_trip_and_to_svg() {
    let dir = tempfile::tempdir().unwrap();
    let png = dir.path().join("pic.png");
    write_test_png(&png);

    for to in [Format::Bmp, Format::Gif, Format::WebP] {
        let raster = convert(&registry(), &ConversionRequest::new(&png, to), &mut |_| {}).unwrap();
        assert_eq!(detect_format(&raster.output).unwrap(), to, "encode {to}");

        // Back to PNG so dimensions stay checkable without format-specific readers.
        let back = convert(
            &registry(),
            &ConversionRequest::new(&raster.output, Format::Png),
            &mut |_| {},
        )
        .unwrap();
        let img = image::open(&back.output).unwrap();
        assert_eq!((img.width(), img.height()), (3, 2), "round-trip {to}");

        let svg = convert(
            &registry(),
            &ConversionRequest::new(&raster.output, Format::Svg),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(detect_format(&svg.output).unwrap(), Format::Svg, "svg {to}");
        let body = std::fs::read_to_string(&svg.output).unwrap();
        assert!(
            body.to_ascii_lowercase().contains("<svg"),
            "svg markup for {to}: {body}"
        );
    }
}
