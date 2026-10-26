//! Why a content file could not be read.

use std::io;
use std::path::PathBuf;

use khalid_core::ParseLocaleError;

use crate::BlockError;

/// Why a content file could not be read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ContentError {
    /// A file or folder could not be read.
    #[error("could not read {}", path.display())]
    Read {
        /// The file or folder, relative to the site folder.
        path: PathBuf,
        /// What the file system reported.
        #[source]
        source: io::Error,
    },
    /// A folder in `content/` is not named after a language.
    #[error("{} is not a language folder: {source}", path.display())]
    NotALanguage {
        /// The folder.
        path: PathBuf,
        /// Why its name is not a language tag.
        source: ParseLocaleError,
    },
    /// A folder entry has no `index.md`.
    #[error("{} is a folder post with no index.md", path.display())]
    FolderWithoutIndex {
        /// The folder.
        path: PathBuf,
    },
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
    /// A block in the body is wrong: an unknown directive, a wrong or missing
    /// attribute, or a block that is never closed.
    #[error("{}, line {line}: {}", path.display(), source.problem())]
    Block {
        /// The file.
        path: PathBuf,
        /// The 1-based line in the file.
        line: usize,
        /// What the block reader found.
        #[source]
        source: BlockError,
    },
}
