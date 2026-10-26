//! What can go wrong while making or keeping derived images.

use std::io;
use std::path::PathBuf;

use khalid_core::SitePathError;

/// Why an image could not be derived or cached.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MediaError {
    /// The cache folder could not be used inside the site folder.
    #[error("the media cache folder could not be used")]
    CachePath(#[from] SitePathError),
    /// A cached file exists but could not be read.
    #[error("could not read {}", path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: io::Error,
    },
    /// A derived file could not be written.
    #[error("could not write {}", path.display())]
    Write {
        /// The file.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: io::Error,
    },
}
