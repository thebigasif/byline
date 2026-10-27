//! The part of an image a crop to another shape keeps.

use crate::{FocalPoint, Pixels};

/// A box inside an image, in pixels from its top left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Crop {
    /// Pixels from the left edge.
    pub left: u32,
    /// Pixels from the top edge.
    pub top: u32,
    /// The box's width.
    pub width: u32,
    /// The box's height.
    pub height: u32,
}

impl Crop {
    /// The box at `left`, `top`, `width` pixels wide and `height` high.
    pub fn new(left: u32, top: u32, width: u32, height: u32) -> Self {
        Self {
            left,
            top,
            width,
            height,
        }
    }

    /// The largest box in an image of `source` pixels whose shape is
    /// `ratio_width` to `ratio_height`, such as 1 to 1 or 16 to 9, centred
    /// on `focal` as far as the edges allow. So a square from a wide photo
    /// keeps the face the focal point marks, not the middle of the photo.
    pub fn around(focal: FocalPoint, source: Pixels, ratio_width: u32, ratio_height: u32) -> Self {
        let (wide, high) = (u64::from(source.width), u64::from(source.height));
        let ratio_width = u64::from(ratio_width.max(1));
        let ratio_height = u64::from(ratio_height.max(1));
        let (width, height) = if wide * ratio_height >= high * ratio_width {
            (
                divide_rounded(high * ratio_width, ratio_height)
                    .max(1)
                    .min(wide),
                high,
            )
        } else {
            (
                wide,
                divide_rounded(wide * ratio_height, ratio_width)
                    .max(1)
                    .min(high),
            )
        };
        Self {
            left: narrow(place(wide, width, focal.x())),
            top: narrow(place(high, height, focal.y())),
            width: narrow(width),
            height: narrow(height),
        }
    }
}

/// Where a box `length` long starts along a side `side` long, so its middle
/// sits `thousandths` of the way along, without passing either end.
fn place(side: u64, length: u64, thousandths: u16) -> u64 {
    let centre = side * u64::from(thousandths) / 1000;
    centre
        .saturating_sub(length / 2)
        .min(side.saturating_sub(length))
}

fn divide_rounded(dividend: u64, divisor: u64) -> u64 {
    (dividend + divisor / 2) / divisor
}

/// Every value here is at most a side of the source, which is a `u32`.
fn narrow(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
