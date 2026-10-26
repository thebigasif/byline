//! What can go wrong while measuring `khalid serve`.

use std::io;
use std::path::PathBuf;

/// `khalid serve` could not be started or measured.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServeError {
    /// The binary could not be started, or a file could not be read or
    /// written.
    #[error("could not use {}", path.display())]
    Io {
        /// The binary or file.
        path: PathBuf,
        /// The cause.
        #[source]
        source: io::Error,
    },
    /// The server stopped, or never said where it answers.
    #[error("the server did not say where it answers")]
    NoAddress,
    /// A request to the server failed.
    #[error("a request to {address} failed")]
    Request {
        /// The page asked for.
        address: String,
        /// The cause.
        #[source]
        source: io::Error,
    },
    /// The edited page never showed the edit.
    #[error("the edit to {address} did not show within {seconds} seconds")]
    EditNotShown {
        /// The page edited.
        address: String,
        /// How long the measure waited.
        seconds: u64,
    },
    /// The system does not report a process's resident memory.
    #[error("this system does not report a process's memory in /proc")]
    NoMemoryReport,
}
