//! Turning and cropping a picture, and where its focal point lands (plan
//! 4.6, decisions E9 and E10).

use std::io::Cursor;

use image::{ImageFormat, Rgb, RgbImage};
use khalid_media::{Crop, CropBox, FocalPoint, Original, PictureEdit, Pixels, Turn};

const RED: Rgb<u8> = Rgb([200, 20, 20]);
const BLUE: Rgb<u8> = Rgb([20, 20, 200]);

/// A blue PNG of `width` by `height` with its top left pixel red.
#[cfg(test)]
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut image = RgbImage::from_pixel(width, height, BLUE);
    image.put_pixel(0, 0, RED);
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).unwrap();
    bytes.into_inner()
}

#[cfg(test)]
fn crop(left: u16, top: u16, width: u16, height: u16) -> CropBox {
    CropBox::try_new(left, top, width, height).unwrap()
}

/// `bytes` decoded, edited as `edit` says, encoded and decoded again.
#[cfg(test)]
fn edited(bytes: &[u8], edit: PictureEdit) -> RgbImage {
    let original = Original::decode(bytes).unwrap();
    let encoded = original.edited(edit).unwrap().reencode().unwrap();
    image::load_from_memory(&encoded).unwrap().to_rgb8()
}

#[test]
fn turns_only_in_quarter_turns() {
    assert_eq!(Turn::from_degrees(90), Some(Turn::Quarter));
    assert_eq!(Turn::from_degrees(270), Some(Turn::ThreeQuarters));
    assert_eq!(Turn::from_degrees(45), None);
    let size = Pixels {
        width: 400,
        height: 200,
    };
    let turned = Turn::Quarter.turned(size);
    assert_eq!((turned.width, turned.height), (200, 400));
}

#[test]
fn a_crop_box_stays_inside_the_picture() {
    assert!(CropBox::try_new(0, 0, 1000, 1000).is_ok());
    assert!(CropBox::try_new(0, 0, 0, 500).is_err());
    assert!(CropBox::try_new(600, 0, 500, 500).is_err());
    let size = Pixels {
        width: 400,
        height: 200,
    };
    let left_half = crop(0, 0, 500, 1000).pixels(size);
    assert_eq!(left_half, Crop::new(0, 0, 200, 200));
    let middle = crop(250, 0, 500, 1000).pixels(size);
    assert_eq!(middle, Crop::new(100, 0, 200, 200));
}

#[test]
fn the_focal_point_moves_with_the_picture() {
    let face = FocalPoint::try_new(200, 300).unwrap();
    let turned = PictureEdit::new(Turn::Quarter, None).focal_point(face);
    assert_eq!((turned.x(), turned.y()), (700, 200));
    let right_half = Some(crop(500, 0, 500, 1000));
    let cropped = PictureEdit::new(Turn::Quarter, right_half).focal_point(face);
    assert_eq!((cropped.x(), cropped.y()), (400, 200));
    let corner = Some(crop(500, 500, 500, 500));
    let cut_off = PictureEdit::new(Turn::None, corner).focal_point(face);
    assert_eq!((cut_off.x(), cut_off.y()), (0, 0));
}

#[test]
fn a_turned_picture_has_its_corners_moved() {
    let quarter = edited(&png(4, 2), PictureEdit::new(Turn::Quarter, None));
    assert_eq!(quarter.dimensions(), (2, 4));
    assert_eq!(quarter.get_pixel(1, 0), &RED, "top left went to top right");
    let back = edited(&png(4, 2), PictureEdit::new(Turn::ThreeQuarters, None));
    assert_eq!(back.get_pixel(0, 3), &RED, "top left went to bottom left");
    let half = edited(&png(4, 2), PictureEdit::new(Turn::Half, None));
    assert_eq!(half.get_pixel(3, 1), &RED, "top left went to bottom right");
}

#[test]
fn a_cropped_picture_is_the_box_it_keeps() {
    let right = PictureEdit::new(Turn::None, Some(crop(500, 0, 500, 1000)));
    let kept = edited(&png(400, 200), right);
    assert_eq!(kept.dimensions(), (200, 200));
    assert_eq!(kept.get_pixel(0, 0), &BLUE, "the red corner was cut off");
    let left = PictureEdit::new(Turn::None, Some(crop(0, 0, 500, 1000)));
    assert_eq!(edited(&png(400, 200), left).get_pixel(0, 0), &RED);
}
