//! The address of one page on the site.

use std::fmt;

/// The path of a page on this site, such as `/blog/what-a-sitemap-does/`
/// or `/robots.txt`, as a reader sees it once percent-encoding is undone.
///
/// It starts with `/`, has no empty, `.` or `..` folder, and has no query,
/// fragment, backslash or control character. That shape means no page can
/// sit at `//other.example`, so a redirect to a page never leaves the site.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PagePath(String);

/// Why a page path was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParsePagePathError {
    /// The path does not start with `/`.
    #[error("the page path {path:?} must start with `/`")]
    Relative {
        /// The path as given.
        path: String,
    },
    /// The path has an empty, `.` or `..` folder.
    #[error("the page path {path:?} has an empty, `.` or `..` folder")]
    Folder {
        /// The path as given.
        path: String,
    },
    /// The path has a character that cannot be part of one.
    #[error("the page path {path:?} has the character {character:?}")]
    Character {
        /// The path as given.
        path: String,
        /// The first character refused.
        character: char,
    },
}

impl PagePath {
    /// Checks `path` against the rules above.
    ///
    /// # Errors
    ///
    /// Returns [`ParsePagePathError::Relative`] without a leading `/`,
    /// [`ParsePagePathError::Character`] for `?`, `#`, `\` or a control
    /// character, and [`ParsePagePathError::Folder`] for an empty, `.` or
    /// `..` folder.
    pub fn try_new(path: &str) -> Result<Self, ParsePagePathError> {
        let owned = || path.to_owned();
        let Some(rest) = path.strip_prefix('/') else {
            return Err(ParsePagePathError::Relative { path: owned() });
        };
        let refused = |c: char| c.is_control() || matches!(c, '?' | '#' | '\\');
        if let Some(character) = path.chars().find(|c| refused(*c)) {
            return Err(ParsePagePathError::Character {
                path: owned(),
                character,
            });
        }
        // The root has no folders. Anything else drops its trailing slash,
        // and each folder left must have a name.
        let folders = rest.strip_suffix('/').unwrap_or(rest);
        let bad = |folder: &str| matches!(folder, "" | "." | "..");
        if !rest.is_empty() && folders.split('/').any(bad) {
            return Err(ParsePagePathError::Folder { path: owned() });
        }
        Ok(Self(path.to_owned()))
    }

    /// The path, such as `/blog/`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PagePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
