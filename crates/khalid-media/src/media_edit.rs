//! A change to a media item's details.

use chrono::{DateTime, Utc};

use crate::{FocalPoint, Folder, MediaError, MediaItem};

/// The most characters of alt text. Screen readers read alt text whole, so
/// a description runs a sentence or two.
const MAX_ALT: usize = 1000;

/// The most characters of a caption.
const MAX_CAPTION: usize = 2000;

/// What to change on an item. A field left `None` stays as it is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MediaEdit {
    /// The `updated_at` of the copy the edit was made on.
    pub updated_at: DateTime<Utc>,
    /// New alt text; empty clears it.
    pub alt: Option<String>,
    /// A new caption; empty clears it.
    pub caption: Option<String>,
    /// A new focal point.
    pub focal_point: Option<FocalPoint>,
    /// A new folder. The address stays the same.
    pub folder: Option<Folder>,
}

/// Why an edit was not saved.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EditMediaError {
    /// The library has no such item.
    #[error("the library has no such item")]
    NotFound,
    /// The item changed after the copy the edit was made on.
    #[error("the item changed after the copy the edit was made on")]
    Stale {
        /// When it last changed.
        updated_at: DateTime<Utc>,
        /// Who changed it.
        updated_by: String,
    },
    /// A text field is longer than it may be.
    #[error("{field} is longer than {limit} characters")]
    TooLong {
        /// `alt` or `caption`.
        field: &'static str,
        /// The most characters it may have.
        limit: usize,
    },
    /// The change could not be written.
    #[error(transparent)]
    Media(#[from] MediaError),
}

impl MediaEdit {
    /// An edit of the copy last changed at `updated_at` that changes
    /// nothing yet.
    pub fn new(updated_at: DateTime<Utc>) -> Self {
        Self {
            updated_at,
            alt: None,
            caption: None,
            focal_point: None,
            folder: None,
        }
    }

    /// `item` with this edit made by `by` at `now`.
    pub(crate) fn applied(
        self,
        item: &MediaItem,
        by: &str,
        now: DateTime<Utc>,
    ) -> Result<MediaItem, EditMediaError> {
        if self.updated_at != item.updated_at {
            return Err(EditMediaError::Stale {
                updated_at: item.updated_at,
                updated_by: item.updated_by.clone(),
            });
        }
        let checked = |field, text: Option<String>, limit| match text {
            Some(text) if text.chars().count() > limit => {
                Err(EditMediaError::TooLong { field, limit })
            }
            text => Ok(text.map(|text| text.trim().to_owned())),
        };
        let alt = checked("alt", self.alt, MAX_ALT)?;
        let caption = checked("caption", self.caption, MAX_CAPTION)?;
        let mut item = item.clone();
        item.alt = alt.unwrap_or(item.alt);
        item.caption = caption.unwrap_or(item.caption);
        item.focal_point = self.focal_point.unwrap_or(item.focal_point);
        item.folder = self.folder.unwrap_or(item.folder);
        item.updated_at = now;
        by.clone_into(&mut item.updated_by);
        Ok(item)
    }
}
