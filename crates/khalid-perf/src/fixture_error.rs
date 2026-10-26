//! What can go wrong while writing a generated site.

use std::io;
use std::path::PathBuf;

/// A generated site could not be written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FixtureError {
    /// The target folder already holds files, which a generated site would
    /// mix with.
    #[error("the folder {} is not empty", path.display())]
    NotEmpty {
        /// The folder.
        path: PathBuf,
    },
    /// A folder or file could not be read or written.
    #[error("could not write {}", path.display())]
    Io {
        /// The file or folder.
        path: PathBuf,
        /// The cause.
        #[source]
        source: io::Error,
    },
}
