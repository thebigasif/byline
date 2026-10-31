//! Turning and cropping a picture from the library (plan 4.6, decisions E9
//! and E10 in `docs/decisions/2026-10-31-editor-emoji-photos-and-edits.md`).

use crate::{Crop, FocalPoint, Pixels};

/// How far to turn a picture, clockwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Turn {
    /// Not at all.
    #[default]
    None,
    /// A quarter turn, 90 degrees.
    Quarter,
    /// Upside down, 180 degrees.
    Half,
    /// Three quarters, 270 degrees, which is a quarter turn to the left.
    ThreeQuarters,
}

impl Turn {
    /// The turn of `degrees` clockwise: 0, 90, 180 or 270, or `None` for
    /// any other angle.
    pub fn from_degrees(degrees: u16) -> Option<Self> {
        match degrees {
            0 => Some(Self::None),
            90 => Some(Self::Quarter),
            180 => Some(Self::Half),
            270 => Some(Self::ThreeQuarters),
            _ => None,
        }
    }

    /// The size of a picture of `size` after this turn.
    pub fn turned(self, size: Pixels) -> Pixels {
        match self {
            Self::None | Self::Half => size,
            Self::Quarter | Self::ThreeQuarters => Pixels {
                width: size.height,
                height: size.width,
            },
        }
    }

    /// Where `point` lands after this turn.
    fn point(self, point: FocalPoint) -> (u16, u16) {
        let (x, y) = (point.x(), point.y());
        match self {
            Self::None => (x, y),
            Self::Quarter => (1000 - y, x),
            Self::Half => (1000 - x, 1000 - y),
            Self::ThreeQuarters => (y, 1000 - x),
        }
    }
}

/// A box to keep, in thousandths of the turned picture's width and height
/// from its top left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CropBox {
    left: u16,
    top: u16,
    width: u16,
    height: u16,
}

/// Why a crop box was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a crop box is at least 1 thousandth wide and high and stays inside the picture")]
pub struct ParseCropBoxError;

impl CropBox {
    /// The box at `left`, `top`, `width` wide and `height` high, each in
    /// thousandths.
    ///
    /// # Errors
    ///
    /// Returns [`ParseCropBoxError`] for an empty box or one that reaches
    /// past an edge.
    pub fn try_new(
        left: u16,
        top: u16,
        width: u16,
        height: u16,
    ) -> Result<Self, ParseCropBoxError> {
        let fits = |start: u16, length: u16| {
            length > 0 && start.checked_add(length).is_some_and(|end| end <= 1000)
        };
        if fits(left, width) && fits(top, height) {
            Ok(Self {
                left,
                top,
                width,
                height,
            })
        } else {
            Err(ParseCropBoxError)
        }
    }

    /// The box in pixels inside a picture of `size`, at least one pixel
    /// each way.
    pub fn pixels(self, size: Pixels) -> Crop {
        let part = |side: u32, thousandths: u16| {
            let scaled = u64::from(side) * u64::from(thousandths) / 1000;
            u32::try_from(scaled).unwrap_or(side)
        };
        let left = part(size.width, self.left).min(size.width.saturating_sub(1));
        let top = part(size.height, self.top).min(size.height.saturating_sub(1));
        let width = part(size.width, self.width)
            .min(size.width.saturating_sub(left))
            .max(1);
        let height = part(size.height, self.height)
            .min(size.height.saturating_sub(top))
            .max(1);
        Crop::new(left, top, width, height)
    }
}

/// What to do to a picture: turn it, then keep a box of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct PictureEdit {
    /// How far to turn it, first.
    pub turn: Turn,
    /// The box to keep after turning, or `None` for all of it.
    pub crop: Option<CropBox>,
}

impl PictureEdit {
    /// An edit that turns by `turn` and keeps `crop`.
    pub fn new(turn: Turn, crop: Option<CropBox>) -> Self {
        Self { turn, crop }
    }

    /// Where `focal` lands in the edited picture, kept inside it when the
    /// crop cut it off.
    pub fn focal_point(self, focal: FocalPoint) -> FocalPoint {
        let (x, y) = self.turn.point(focal);
        let Some(crop) = self.crop else {
            return FocalPoint::try_new(x, y).unwrap_or_default();
        };
        let inside = |at: u16, start: u16, length: u16| {
            let from = u32::from(at.saturating_sub(start)) * 1000 / u32::from(length.max(1));
            u16::try_from(from.min(1000)).unwrap_or(1000)
        };
        let x = inside(x, crop.left, crop.width);
        let y = inside(y, crop.top, crop.height);
        FocalPoint::try_new(x, y).unwrap_or_default()
    }
}
