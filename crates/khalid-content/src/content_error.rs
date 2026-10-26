//! Why a content file could not be read.

use std::path::PathBuf;

/// Why a content file could not be read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ContentError {
    /// The file does not start with a `---` line.
    #[error("{} does not start with front matter between two --- lines", path.display())]
    NoFrontMatter {
        /// The file.
        path: PathBuf,
    },
    /// The opening `---` line has no closing one.
    #[error("{} opens its front matter with --- but never closes it", path.display())]
    UnclosedFrontMatter {
        /// The file.
        path: PathBuf,
    },
    /// A field is unknown, has the wrong kind of value, or the YAML is broken.
    #[error("{}, line {line}, field {field:?}: {problem}", path.display())]
    Field {
        /// The file.
        path: PathBuf,
        /// The 1-based line in the file.
        line: usize,
        /// The field on that line, or empty when there is none.
        field: String,
        /// What is wrong, in one phrase.
        problem: String,
        /// What the YAML reader found.
        #[source]
        source: Box<serde_saphyr::Error>,
    },
}
