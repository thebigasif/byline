//! One file in the media library and the details kept with it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ContentKey, FileName, FocalPoint, Folder, MediaType};

/// An image's size in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pixels {
    /// The width.
    pub width: u32,
    /// The height.
    pub height: u32,
}

/// A file in the media library with its details, as kept in
/// `media/<id>/item.toml` (ADR "the media library keeps each file with its
/// details").
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MediaItem {
    /// A hash of the uploaded bytes, which names the item's folder.
    pub id: ContentKey,
    /// The name it is served under.
    pub name: FileName,
    /// The library folder it is filed in.
    pub folder: Folder,
    /// Its type, read from its bytes.
    pub media_type: MediaType,
    /// Its size in bytes.
    pub bytes: u64,
    /// Its size in pixels, for images only.
    pub pixels: Option<Pixels>,
    /// What an image shows, for readers who cannot see it. Empty until set.
    pub alt: String,
    /// The line shown under it. Empty for none.
    pub caption: String,
    /// The part of an image every crop keeps.
    pub focal_point: FocalPoint,
    /// When it was uploaded.
    pub uploaded_at: DateTime<Utc>,
    /// Who uploaded it: a staff member or an integration key's name.
    pub uploaded_by: String,
    /// When its details last changed.
    pub updated_at: DateTime<Utc>,
    /// Who changed them.
    pub updated_by: String,
}

/// `item.toml` as written. The identity is the folder's name, so it is not
/// repeated inside.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemFile {
    name: FileName,
    folder: Folder,
    #[serde(rename = "type")]
    media_type: MediaType,
    bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<u32>,
    alt: String,
    caption: String,
    focal_x: u16,
    focal_y: u16,
    uploaded_at: DateTime<Utc>,
    uploaded_by: String,
    updated_at: DateTime<Utc>,
    updated_by: String,
}

/// An `item.toml` that could not be read.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ParseItemError {
    /// The text is not TOML, or a field is missing or wrong.
    #[error("the item file is not valid: {}", source.message())]
    Toml {
        /// What the TOML reader found.
        #[source]
        source: Box<toml::de::Error>,
    },
    /// Only one of `width` and `height` is set.
    #[error("the item file sets one of width and height without the other")]
    HalfSize,
    /// The focal point lies outside the image.
    #[error("the item file's focal_x and focal_y must each be 0 to 1000")]
    FocalPoint,
}

impl MediaItem {
    /// Reads the item `id` from the text of its `item.toml`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseItemError`] naming the first thing wrong.
    pub fn from_toml(id: ContentKey, text: &str) -> Result<Self, ParseItemError> {
        let file: ItemFile = toml::from_str(text).map_err(|source| ParseItemError::Toml {
            source: Box::new(source),
        })?;
        let pixels = match (file.width, file.height) {
            (Some(width), Some(height)) => Some(Pixels { width, height }),
            (None, None) => None,
            _ => return Err(ParseItemError::HalfSize),
        };
        let focal_point = FocalPoint::try_new(file.focal_x, file.focal_y)
            .map_err(|_| ParseItemError::FocalPoint)?;
        Ok(Self {
            id,
            name: file.name,
            folder: file.folder,
            media_type: file.media_type,
            bytes: file.bytes,
            pixels,
            alt: file.alt,
            caption: file.caption,
            focal_point,
            uploaded_at: file.uploaded_at,
            uploaded_by: file.uploaded_by,
            updated_at: file.updated_at,
            updated_by: file.updated_by,
        })
    }

    /// The text of the item's `item.toml`.
    pub fn to_toml(&self) -> String {
        let file = ItemFile {
            name: self.name.clone(),
            folder: self.folder.clone(),
            media_type: self.media_type,
            bytes: self.bytes,
            width: self.pixels.map(|pixels| pixels.width),
            height: self.pixels.map(|pixels| pixels.height),
            alt: self.alt.clone(),
            caption: self.caption.clone(),
            focal_x: self.focal_point.x(),
            focal_y: self.focal_point.y(),
            uploaded_at: self.uploaded_at,
            uploaded_by: self.uploaded_by.clone(),
            updated_at: self.updated_at,
            updated_by: self.updated_by.clone(),
        };
        // Every field is a string, a number or a time, which TOML always
        // writes, so this never fails.
        toml::to_string(&file).unwrap_or_default()
    }
}
