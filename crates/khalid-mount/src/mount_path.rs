//! The folder of the main site the blog is mounted at.

use crate::MountError;

/// The folder of the main site the blog lives under, such as `/blog` or
/// `/resources/blog`: never the root, no trailing slash, and only letters,
/// digits, `-`, `_`, `.` and `~`, so it is safe to paste into any config.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MountPath(String);

impl MountPath {
    /// Checks `text` against the rules above.
    ///
    /// # Errors
    ///
    /// Returns [`MountError::Root`] for `/`, [`MountError::Character`] for
    /// a character outside the safe set, and [`MountError::Shape`] for any
    /// other path that is not a folder.
    pub fn try_new(text: &str) -> Result<Self, MountError> {
        if text == "/" {
            return Err(MountError::Root);
        }
        let safe = |c: char| c.is_ascii_alphanumeric() || "-_.~/".contains(c);
        if let Some(character) = text.chars().find(|c| !safe(*c)) {
            return Err(MountError::Character { character });
        }
        let folders = text.strip_prefix('/').ok_or(MountError::Shape)?.split('/');
        if folders.into_iter().any(|f| matches!(f, "" | "." | "..")) {
            return Err(MountError::Shape);
        }
        Ok(Self(text.to_owned()))
    }

    /// The path, such as `/blog`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
