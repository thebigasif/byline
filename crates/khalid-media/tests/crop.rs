//! A focal point drives every crop, and an uploaded image is stored again
//! in its own format, upright and without its hidden data (RULES.md
//! BL-SEC-6, plan section 5.15).

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageEncoder, ImageFormat, Rgba, RgbaImage};
use khalid_media::{Crop, FocalPoint, Format, Original, Pixels};

/// An image `width` by `height`, red on the left half and blue on the right.
#[cfg(test)]
fn halves(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, _| {
        if x < width / 2 {
            Rgba([220, 20, 20, 255])
        } else {
            Rgba([20, 20, 220, 255])
        }
    })
}

/// `image` as an opaque file in `format`; JPEG has no see-through pixels.
#[cfg(test)]
fn encoded(image: &RgbaImage, format: ImageFormat) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    let opaque = DynamicImage::ImageRgba8(image.clone()).into_rgb8();
    opaque.write_to(&mut bytes, format).unwrap();
    bytes.into_inner()
}

#[cfg(test)]
fn focus(x: u16, y: u16) -> FocalPoint {
    FocalPoint::try_new(x, y).unwrap()
}

#[test]
fn a_crop_is_the_largest_box_of_the_shape_centred_on_the_focal_point() {
    let photo = Pixels {
        width: 1600,
        height: 900,
    };
    let square = |point| Crop::around(point, photo, 1, 1);
    assert_eq!(square(FocalPoint::default()), Crop::new(350, 0, 900, 900));
    assert_eq!(square(focus(620, 500)), Crop::new(542, 0, 900, 900));
    assert_eq!(square(focus(1000, 0)), Crop::new(700, 0, 900, 900));
    assert_eq!(square(focus(0, 1000)), Crop::new(0, 0, 900, 900));
    let tall = Crop::around(focus(500, 200), photo, 9, 16);
    assert_eq!(tall, Crop::new(547, 0, 506, 900));
    let banner = Crop::around(focus(500, 900), photo, 4, 1);
    assert_eq!(banner, Crop::new(0, 500, 1600, 400));
}

#[test]
fn a_cropped_variant_keeps_the_side_the_focal_point_is_on() {
    let original = Original::decode(&encoded(&halves(200, 100), ImageFormat::Png)).unwrap();
    let centre_of = |point| {
        let bytes = original.encode_cropped(Format::Png, 40, 40, point).unwrap();
        let image = image::load_from_memory(&bytes).unwrap().into_rgba8();
        assert_eq!(image.dimensions(), (40, 40));
        image.get_pixel(20, 20).0
    };
    assert!(centre_of(focus(900, 500))[2] > 200, "right is blue");
    assert!(centre_of(focus(100, 500))[0] > 200, "left is red");
}

#[test]
fn an_upload_is_stored_again_in_its_own_format() {
    for format in [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::WebP] {
        let original = Original::decode(&encoded(&halves(64, 32), format)).unwrap();
        let stored = original.reencode().unwrap();
        assert_eq!(image::guess_format(&stored).unwrap(), format);
        let back = image::load_from_memory(&stored).unwrap();
        assert_eq!((back.width(), back.height()), (64, 32));
    }
}

/// A JPEG of `halves(40, 20)` whose EXIF data says to turn it a quarter
/// clockwise to show it upright, as a phone camera writes.
#[cfg(test)]
fn sideways_photo() -> Vec<u8> {
    let rgb: Vec<u8> = halves(40, 20)
        .pixels()
        .flat_map(|pixel| [pixel.0[0], pixel.0[1], pixel.0[2]])
        .collect();
    // A TIFF block, big-endian, with one entry: orientation (0x0112), a
    // short, value 6, which means "turn a quarter clockwise".
    let exif = b"MM\x00\x2A\x00\x00\x00\x08\x00\x01\x01\x12\x00\x03\x00\x00\x00\x01\x00\x06\x00\x00\x00\x00\x00\x00";
    let mut bytes = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut bytes, 90);
    encoder.set_exif_metadata(exif.to_vec()).unwrap();
    encoder
        .write_image(&rgb, 40, 20, image::ExtendedColorType::Rgb8)
        .unwrap();
    bytes
}

#[test]
fn a_sideways_photo_is_turned_upright_and_loses_its_exif_data() {
    let upload = sideways_photo();
    assert!(
        upload.windows(4).any(|w| w == b"Exif"),
        "the test photo has EXIF"
    );
    let original = Original::decode(&upload).unwrap();
    assert_eq!((original.width(), original.height()), (20, 40));
    let stored = original.reencode().unwrap();
    assert!(!stored.windows(4).any(|w| w == b"Exif"), "EXIF is dropped");
    let upright = image::load_from_memory(&stored).unwrap().into_rgba8();
    assert_eq!(upright.dimensions(), (20, 40));
    assert!(
        upright.get_pixel(10, 5).0[0] > 150,
        "red, the left, is now on top"
    );
    assert!(
        upright.get_pixel(10, 35).0[2] > 150,
        "blue is at the bottom"
    );
}
