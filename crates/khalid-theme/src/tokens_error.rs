//! Why `theme/tokens.toml` could not be read or written.

use std::io;
use std::path::PathBuf;

use khalid_core::SitePathError;

/// Why `theme/tokens.toml` could not be read or written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TokensError {
    /// A value is not a colour, a font list or a length, or a field is
    /// one the renderer does not know.
    #[error("theme/tokens.toml has a mistake")]
    Parse {
        /// The line of the mistake, counting from 1, when it is known.
        line: Option<usize>,
        /// What the TOML reader reported.
        #[source]
        source: toml::de::Error,
    },
    /// The path leads outside the site folder, or could not be resolved.
    #[error("theme/tokens.toml is not a safe path inside the site folder")]
    Path {
        /// Why the path was refused.
        #[from]
        source: SitePathError,
    },
    /// The file exists but could not be read.
    #[error("could not read {}", path.display())]
    Read {
        /// The file being read.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: io::Error,
    },
    /// The file could not be written.
    #[error("could not write {}", path.display())]
    Write {
        /// The file being written.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: io::Error,
    },
}

impl TokensError {
    /// The line of a mistake in the file, when there is one.
    pub fn line(&self) -> Option<usize> {
        match self {
            Self::Parse { line, .. } => *line,
            Self::Path { .. } | Self::Read { .. } | Self::Write { .. } => None,
        }
    }
}
