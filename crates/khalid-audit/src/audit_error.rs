//! Why the audit could not start.

use std::io;
use std::path::PathBuf;

/// Why the audit could not start. Once it has started, an address that
/// fails is an issue in the report, never an error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AuditError {
    /// The built folder to crawl is not there.
    #[error("{} is not a folder", folder.display())]
    Folder {
        /// The folder as given.
        folder: PathBuf,
    },
    /// The folder's `_redirects` file is there but cannot be read.
    #[error("{} could not be read", file.display())]
    Redirects {
        /// The file.
        file: PathBuf,
        /// What reading it reported.
        #[source]
        source: io::Error,
    },
}
