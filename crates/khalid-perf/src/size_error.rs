//! What can go wrong while measuring the stylesheet and the admin.

use std::io;
use std::path::PathBuf;

/// A built file could not be found or read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SizeError {
    /// A file the page links is not where the link says.
    #[error("{} is missing", path.display())]
    Missing {
        /// Where it was looked for.
        path: PathBuf,
    },
    /// A file could not be read.
    #[error("could not read {}", path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// The cause.
        #[source]
        source: io::Error,
    },
}
