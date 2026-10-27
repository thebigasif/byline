//! The part of an image that every crop keeps.

/// The most thousandths: the right or bottom edge.
const EDGE: u16 = 1000;

/// A point in an image, in thousandths of its width from the left and of its
/// height from the top. `x = 500, y = 500` is the centre, the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocalPoint {
    x: u16,
    y: u16,
}

/// A focal point that lies outside the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a focal point's x and y are each from 0 to 1000")]
pub struct ParseFocalPointError;

impl FocalPoint {
    /// The point `x` thousandths across and `y` thousandths down.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFocalPointError`] when either is above 1000.
    pub fn try_new(x: u16, y: u16) -> Result<Self, ParseFocalPointError> {
        if x > EDGE || y > EDGE {
            return Err(ParseFocalPointError);
        }
        Ok(Self { x, y })
    }

    /// Thousandths of the width from the left edge.
    pub fn x(self) -> u16 {
        self.x
    }

    /// Thousandths of the height from the top edge.
    pub fn y(self) -> u16 {
        self.y
    }
}

impl Default for FocalPoint {
    /// The centre.
    fn default() -> Self {
        Self { x: 500, y: 500 }
    }
}
