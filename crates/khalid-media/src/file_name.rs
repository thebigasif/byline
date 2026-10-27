//! The name a media item is saved and served under.

use khalid_core::{Locale, Slug, SlugMode};
use serde::{Deserialize, Serialize};

use crate::MediaType;

/// The stem used when an upload's name has no letters or digits to keep.
const FALLBACK_STEM: &str = "file";

/// A file name such as `choosing-a-crm.jpg`: an ASCII slug of the uploaded
/// name, then the extension of the type read from the bytes. It is safe in
/// a path and an address, and search engines read its words.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FileName(String);

/// A stored file name that is not a slug with an accepted extension.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("a file name is a slug and the extension of an accepted type, such as photo.jpg")]
pub struct ParseFileNameError;

impl FileName {
    /// The name for an upload sent as `upload` whose bytes are `media_type`.
    /// Folders in `upload` and its extension are dropped, and the rest is
    /// turned into a slug with `locale`'s spelling rules, so `Größe.png`
    /// becomes `groesse.png` in German.
    pub fn new(upload: &str, media_type: MediaType, locale: &Locale) -> Self {
        let base = upload.rsplit(['/', '\\']).next().unwrap_or(upload);
        let stem = base.rsplit_once('.').map_or(base, |(stem, _)| stem);
        let slug = Slug::from_title(stem, locale, SlugMode::Ascii)
            .ok()
            .filter(|slug| slug.as_str().is_ascii());
        let stem = slug.as_ref().map_or(FALLBACK_STEM, Slug::as_str);
        Self(format!("{stem}.{}", media_type.extension()))
    }

    /// The name, such as `choosing-a-crm.jpg`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for FileName {
    type Error = ParseFileNameError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let (stem, extension) = text.rsplit_once('.').ok_or(ParseFileNameError)?;
        let known = MediaType::ALL
            .iter()
            .any(|media_type| media_type.extension() == extension);
        if !known || Slug::try_new(stem).is_err() {
            return Err(ParseFileNameError);
        }
        Ok(Self(text))
    }
}

impl From<FileName> for String {
    fn from(name: FileName) -> Self {
        name.0
    }
}
