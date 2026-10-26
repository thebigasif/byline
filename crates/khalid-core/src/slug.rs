//! The last part of a post's address, checked once.

use std::fmt;
use std::str::FromStr;

/// A post's slug: lowercase letters a to z, digits and single hyphens.
///
/// A `Slug` is always valid, so code that takes one never checks it again
/// (RULES.md BL-TYP-3). The rules keep the address readable and the same on
/// every server and file system: no capitals, no spaces, no accents, no hyphen
/// at either end and never two hyphens in a row.
///
/// # Examples
///
/// ```
/// use khalid_core::Slug;
///
/// let slug = Slug::try_new("why-internal-links-matter")?;
/// assert_eq!(slug.as_str(), "why-internal-links-matter");
/// assert!(Slug::try_new("Why Links Matter").is_err());
/// # Ok::<(), khalid_core::ParseSlugError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slug(String);

impl Slug {
    /// The longest slug, in bytes.
    ///
    /// A post file is named `<date>-<slug>.md`, and most file systems allow 255
    /// bytes in a name, so 200 leaves room for the date and the extension.
    pub const MAX_LEN: usize = 200;

    /// Checks `text` and wraps it.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseSlugError`] naming the first rule `text` breaks.
    pub fn try_new(text: &str) -> Result<Self, ParseSlugError> {
        if text.is_empty() {
            return Err(ParseSlugError::Empty);
        }
        if text.len() > Self::MAX_LEN {
            return Err(ParseSlugError::TooLong { len: text.len() });
        }
        let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
        if let Some((position, character)) = text.char_indices().find(|&(_, c)| !allowed(c)) {
            return Err(ParseSlugError::InvalidCharacter {
                character,
                position,
            });
        }
        if text.starts_with('-') || text.ends_with('-') {
            return Err(ParseSlugError::EdgeHyphen);
        }
        if let Some(position) = text.find("--") {
            return Err(ParseSlugError::DoubleHyphen { position });
        }
        Ok(Self(text.to_owned()))
    }

    /// The slug as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Slug {
    type Err = ParseSlugError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_new(text)
    }
}

impl AsRef<str> for Slug {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The rule a slug broke.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseSlugError {
    /// The text was empty.
    #[error("the slug is empty")]
    Empty,
    /// The text was longer than [`Slug::MAX_LEN`] bytes.
    #[error("the slug is {len} bytes long, and the limit is {max}", max = Slug::MAX_LEN)]
    TooLong {
        /// The length of the text, in bytes.
        len: usize,
    },
    /// The text held something other than `a` to `z`, `0` to `9` or `-`.
    #[error(
        "the slug has {character:?} at byte {position}, and only lowercase letters a to z, digits and hyphens are allowed"
    )]
    InvalidCharacter {
        /// The first character that is not allowed.
        character: char,
        /// Its byte offset in the text.
        position: usize,
    },
    /// The text started or ended with a hyphen.
    #[error("the slug starts or ends with a hyphen")]
    EdgeHyphen,
    /// The text held two hyphens in a row.
    #[error("the slug has two hyphens in a row at byte {position}")]
    DoubleHyphen {
        /// The byte offset of the first of the two hyphens.
        position: usize,
    },
}
