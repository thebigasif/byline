//! The folder a media item is filed in, such as `guides/2026`.

use khalid_core::Slug;
use serde::{Deserialize, Serialize};

/// The most folders one can sit inside another.
const MAX_DEPTH: usize = 5;

/// A folder in the media library: empty for the top, or up to five slugs
/// joined by `/`. A folder is a detail of an item, not a place on disk, so
/// moving an item never changes its address.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Folder(String);

/// A folder name that is not up to five slugs joined by `/`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("a folder is up to five names of lowercase letters, digits and hyphens, joined by /")]
pub struct ParseFolderError;

impl Folder {
    /// The top of the library.
    pub fn root() -> Self {
        Self::default()
    }

    /// Reads `text`, ignoring a `/` at either end.
    ///
    /// # Errors
    ///
    /// Returns [`ParseFolderError`] when a part is not a slug, such as `..`
    /// or an empty part between two `/`, or there are more than five parts.
    pub fn try_new(text: &str) -> Result<Self, ParseFolderError> {
        let text = text.trim_matches('/');
        if text.is_empty() {
            return Ok(Self::root());
        }
        let parts: Vec<&str> = text.split('/').collect();
        let slugs = parts.iter().all(|part| Slug::try_new(part).is_ok());
        if !slugs || parts.len() > MAX_DEPTH {
            return Err(ParseFolderError);
        }
        Ok(Self(text.to_owned()))
    }

    /// The folder as written, such as `guides/2026`, or empty for the top.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Folder {
    type Error = ParseFolderError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::try_new(&text)
    }
}

impl From<Folder> for String {
    fn from(folder: Folder) -> Self {
        folder.0
    }
}
