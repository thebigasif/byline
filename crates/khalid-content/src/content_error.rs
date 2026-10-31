//! Why a content file could not be read.

use std::io;
use std::path::PathBuf;

use khalid_core::{Locale, ParseLocaleError};

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
    /// A `[[slug]]` matches no post, page or glossary term in its language.
    #[error(
        "{}, line {line}: [[{target}]] matches no post, page or glossary term in {language}",
        path.display()
    )]
    Wikilink {
        /// The file.
        path: PathBuf,
        /// The 1-based line in the file.
        line: usize,
        /// The slug as written between the brackets.
        target: String,
        /// The file's language, where the target was looked for.
        language: Locale,
    },
    /// A `::snippet{slug="..."}` names no snippet in its language.
    #[error(
        "{}, line {line}: there is no snippet called {slug} in {language}",
        path.display()
    )]
    Snippet {
        /// The file.
        path: PathBuf,
        /// The 1-based line in the file.
        line: usize,
        /// The slug the block names.
        slug: String,
        /// The file's language, where the snippet was looked for.
        language: Locale,
    },
    /// A formula between `$` signs cannot be drawn.
    #[error("{}, line {line}: the maths cannot be drawn: {problem}", path.display())]
    Maths {
        /// The file.
        path: PathBuf,
        /// The 1-based line in the file the formula starts on.
        line: usize,
        /// What the maths reader found, in a phrase.
        problem: String,
    },
}
