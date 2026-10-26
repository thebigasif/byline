//! The last part of a post's address, checked once.

use std::fmt;
use std::str::FromStr;

/// A post's slug: by default lowercase letters a to z, digits and single hyphens.
///
/// A `Slug` is always valid, so code that takes one never checks it again
/// (RULES.md BL-TYP-3). The rules keep the address readable and the same on
/// every server and file system: no capitals, no spaces, no accents, no hyphen
/// at either end and never two hyphens in a row. [`Slug::try_new_unicode`]
/// allows letters from any script, and [`Slug::from_title`] makes a slug from a
/// title in either form.
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

    /// Checks `text` against the strict ASCII rules and wraps it.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseSlugError`] naming the first rule `text` breaks.
    pub fn try_new(text: &str) -> Result<Self, ParseSlugError> {
        let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
        Self::check(text, text.len(), allowed)
    }

    /// Checks `text` against the Unicode rules, for sites with
    /// `slugs = "unicode"` and for scripts with no ASCII form: lowercase or
    /// caseless letters and digits from any script, in Unicode NFC form, and
    /// single hyphens. The limit applies to the percent-encoded length, which
    /// is what a URL carries.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseSlugError`] naming the first rule `text` breaks.
    pub fn try_new_unicode(text: &str) -> Result<Self, ParseSlugError> {
        if !unicode_normalization::is_nfc(text) {
            return Err(ParseSlugError::NotNormalized);
        }
        let allowed = |c: char| c == '-' || (c.is_alphanumeric() && !c.is_uppercase());
        Self::check(text, encoded_len(text), allowed)
    }

    /// The rules both kinds of slug share.
    fn check(
        text: &str,
        len: usize,
        allowed: impl Fn(char) -> bool,
    ) -> Result<Self, ParseSlugError> {
        if text.is_empty() {
            return Err(ParseSlugError::Empty);
        }
        if len > Self::MAX_LEN {
            return Err(ParseSlugError::TooLong { len });
        }
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

/// The length of `text` in a URL: ASCII bytes count once, every other byte is
/// percent-encoded as three.
pub(crate) fn encoded_len(text: &str) -> usize {
    text.bytes().map(|b| if b.is_ascii() { 1 } else { 3 }).sum()
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

impl<'de> serde::Deserialize<'de> for Slug {
    /// Reads a slug from text, accepting the ASCII form or, when the text is
    /// not ASCII, the Unicode form.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let slug = if text.is_ascii() {
            Self::try_new(&text)
        } else {
            Self::try_new_unicode(&text)
        };
        slug.map_err(serde::de::Error::custom)
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
    /// The text was longer than [`Slug::MAX_LEN`] bytes, counting percent
    /// encoding for a Unicode slug.
    #[error("the slug is {len} bytes long, and the limit is {max}", max = Slug::MAX_LEN)]
    TooLong {
        /// The length of the text, in bytes.
        len: usize,
    },
    /// The text held something other than `a` to `z`, `0` to `9` or `-`.
    #[error(
        "the slug has {character:?} at byte {position}, but only a to z, 0 to 9 and hyphens are allowed"
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
    /// A Unicode slug was not in Unicode NFC form.
    #[error("the slug is not in Unicode NFC form, so the same word could have two addresses")]
    NotNormalized,
    /// The text held two hyphens in a row.
    #[error("the slug has two hyphens in a row at byte {position}")]
    DoubleHyphen {
        /// The byte offset of the first of the two hyphens.
        position: usize,
    },
}
