//! Why a share image could not be drawn.

/// Why a share image could not be drawn.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ShareImageError {
    /// The card's SVG could not be read back for drawing.
    #[error("could not lay out the share image")]
    Layout {
        /// What the SVG reader reported.
        #[source]
        source: resvg::usvg::Error,
    },
    /// There was no memory for a 1200 by 630 canvas.
    #[error("could not make a canvas for the share image")]
    Canvas,
    /// The drawn image could not be encoded as PNG.
    #[error("could not encode the share image as PNG")]
    Encode {
        /// What the PNG encoder reported.
        #[source]
        source: png::EncodingError,
    },
}
