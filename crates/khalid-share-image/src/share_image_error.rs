//! Why a share image could not be drawn.

use std::path::PathBuf;

/// Why a share image could not be drawn.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ShareImageError {
    /// There was no memory for a 1200 by 630 canvas.
    #[error("could not make a canvas for the share image")]
    Canvas,
    /// The drawn image could not be encoded as PNG.
    #[error("could not encode the share image as PNG")]
    Encode,
    /// The cache folder could not be used inside the site folder.
    #[error("the share image cache folder could not be used")]
    CachePath(#[from] khalid_core::SitePathError),
    /// A cached file exists but cannot be read.
    #[error("could not read {}", path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: std::io::Error,
    },
    /// A drawn card could not be written to the cache.
    #[error("could not write {}", path.display())]
    Write {
        /// The file.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: std::io::Error,
    },
}
