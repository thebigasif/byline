//! Why `routes.toml` could not be read.

use std::io;
use std::path::PathBuf;

/// Why `routes.toml` could not be read. Every mistake in the file names its
/// line and field, the way `site.toml` mistakes do.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RoutesError {
    /// The file exists but could not be read.
    #[error("could not read {}", path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: io::Error,
    },
    /// The file is not valid TOML, or a field is unknown or of the wrong kind.
    #[error("{}, line {line}, field {field:?}: {}", path.display(), source.message())]
    Parse {
        /// The file.
        path: PathBuf,
        /// The 1-based line of the mistake.
        line: usize,
        /// The field, or empty when the mistake is not in one field.
        field: String,
        /// What the TOML reader found.
        #[source]
        source: Box<toml::de::Error>,
    },
    /// A field is readable but its value breaks a rule, such as a permalink
    /// without `{slug}`.
    #[error("{}, line {line}, field {field:?}: {problem}", path.display())]
    Field {
        /// The file.
        path: PathBuf,
        /// The 1-based line of the field.
        line: usize,
        /// The field.
        field: String,
        /// The rule it breaks.
        problem: String,
    },
}
