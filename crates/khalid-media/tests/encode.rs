//! Decoding by content, sizes that keep their shape, and every format.

use std::io::Cursor;

use image::{ImageFormat, Rgba, RgbaImage};
use khalid_media::{Format, MediaError, Original};

/// A 160 by 90 gradient in `format`, made in the test so no binary fixture
/// is needed, with the opacity `opacity` gives each column.
#[cfg(test)]
fn gradient(format: ImageFormat, opacity: fn(u32) -> u8) -> Vec<u8> {
    let image = RgbaImage::from_fn(160, 90, |x, y| {
        Rgba([
            u8::try_from(x).unwrap(),
            u8::try_from(y * 2).unwrap(),
            120,
            opacity(x),
        ])
    });
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, format).unwrap();
    bytes.into_inner()
}

#[cfg(test)]
fn opaque(_: u32) -> u8 {
    255
}

#[cfg(test)]
fn right_half_clear(x: u32) -> u8 {
    if x >= 80 { 0 } else { 255 }
}

#[test]
fn an_original_is_decoded_by_its_bytes_and_keeps_its_own_fallback() {
    let png = Original::decode(&gradient(ImageFormat::Png, opaque)).unwrap();
    assert_eq!((png.width(), png.height()), (160, 90));
    assert_eq!(png.fallback(), Format::Png, "a PNG stays a PNG");
    let webp = |opacity| Original::decode(&gradient(ImageFormat::WebP, opacity)).unwrap();
    assert_eq!(webp(opaque).fallback(), Format::Jpeg);
    assert_eq!(webp(right_half_clear).fallback(), Format::Png);
}

#[test]
fn a_file_that_is_not_an_image_is_refused() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>";
    assert!(matches!(
        Original::decode(svg),
        Err(MediaError::UnsupportedImage)
    ));
    let mut broken = gradient(ImageFormat::Png, opaque);
    broken.truncate(60);
    assert!(matches!(
        Original::decode(&broken),
        Err(MediaError::Decode { .. })
    ));
}

#[test]
fn a_variant_keeps_the_shape() {
    let original = Original::decode(&gradient(ImageFormat::Png, opaque)).unwrap();
    let variant = original.variant(Format::Webp, 80);
    assert_eq!((variant.width, variant.height), (80, 45));
    assert_eq!(
        original.variant(Format::Webp, 1).height,
        1,
        "never zero pixels tall"
    );
}

#[test]
fn every_format_encodes_at_the_variant_size() {
    let original = Original::decode(&gradient(ImageFormat::Png, right_half_clear)).unwrap();
    for (format, expected) in [
        (Format::Webp, ImageFormat::WebP),
        (Format::Jpeg, ImageFormat::Jpeg),
        (Format::Png, ImageFormat::Png),
    ] {
        let bytes = original.encode(&original.variant(format, 64)).unwrap();
        assert_eq!(image::guess_format(&bytes).unwrap(), expected);
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (64, 36), "{format:?}");
    }
}

#[test]
fn webp_is_smaller_than_png() {
    let original = Original::decode(&gradient(ImageFormat::Png, opaque)).unwrap();
    let size = |format| {
        original
            .encode(&original.variant(format, 160))
            .unwrap()
            .len()
    };
    assert!(size(Format::Webp) < size(Format::Png));
}
