//! The language a piece of content is written in.

use std::fmt;
use std::str::FromStr;

/// A language tag: a language, an optional script and an optional region.
///
/// It covers the part of BCP 47 that websites use, such as `en`, `de`,
/// `pt-BR`, `zh-Hant-TW` and `es-419`, and it always holds the standard case:
/// lowercase language, title-case script, uppercase region. Content folders,
/// `hreflang` and the `lang` attribute all take their value from it
/// (RULES.md BL-TYP-3).
///
/// # Examples
///
/// ```
/// use khalid_core::Locale;
///
/// let locale = Locale::try_new("pt-br")?;
/// assert_eq!(locale.as_str(), "pt-BR");
/// assert_eq!(locale.language(), "pt");
/// # Ok::<(), khalid_core::ParseLocaleError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Locale(String);

impl Locale {
    /// Checks `tag` and writes it in standard case.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseLocaleError`] when `tag` is empty, when its language
    /// is not two or three letters, or when a later part is not a script or a
    /// region in the right place.
    pub fn try_new(tag: &str) -> Result<Self, ParseLocaleError> {
        if tag.is_empty() {
            return Err(ParseLocaleError::Empty);
        }
        let mut parts = tag.split('-');
        let language = parts.next().unwrap_or_default();
        if !(2..=3).contains(&language.len()) || !language.chars().all(|c| c.is_ascii_alphabetic())
        {
            return Err(ParseLocaleError::Language);
        }
        let mut normal = language.to_ascii_lowercase();
        let mut stage = Stage::Script;
        for part in parts {
            stage = match (stage, Subtag::of(part)) {
                (Stage::Script, Some(Subtag::Script)) => {
                    normal.push('-');
                    let mut chars = part.chars();
                    normal.extend(chars.next().map(|c| c.to_ascii_uppercase()));
                    normal.push_str(&chars.as_str().to_ascii_lowercase());
                    Stage::Region
                }
                (Stage::Script | Stage::Region, Some(Subtag::Region)) => {
                    normal.push('-');
                    normal.push_str(&part.to_ascii_uppercase());
                    Stage::Done
                }
                _ => {
                    return Err(ParseLocaleError::Subtag {
                        subtag: part.to_owned(),
                    });
                }
            };
        }
        Ok(Self(normal))
    }

    /// The tag in standard case, such as `pt-BR`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The language alone, such as `pt` for `pt-BR`.
    pub fn language(&self) -> &str {
        self.0
            .split_once('-')
            .map_or(self.0.as_str(), |(language, _)| language)
    }
}

/// Which part of the tag may come next.
#[derive(Clone, Copy)]
enum Stage {
    Script,
    Region,
    Done,
}

/// What a part after the language can be.
enum Subtag {
    Script,
    Region,
}

impl Subtag {
    fn of(part: &str) -> Option<Self> {
        let letters = part.chars().all(|c| c.is_ascii_alphabetic());
        match part.len() {
            4 if letters => Some(Self::Script),
            2 if letters => Some(Self::Region),
            3 if part.chars().all(|c| c.is_ascii_digit()) => Some(Self::Region),
            _ => None,
        }
    }
}

impl FromStr for Locale {
    type Err = ParseLocaleError;

    fn from_str(tag: &str) -> Result<Self, Self::Err> {
        Self::try_new(tag)
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The part of a language tag that broke the rules.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseLocaleError {
    /// The tag was empty.
    #[error("the language tag is empty")]
    Empty,
    /// The first part was not two or three letters.
    #[error("the language must be a two- or three-letter code such as en or de")]
    Language,
    /// A later part was neither a script nor a region, or came out of order.
    #[error(
        "{subtag:?} is not a script such as Hant or a region such as BR or 419 in the right place"
    )]
    Subtag {
        /// The part that broke the rules.
        subtag: String,
    },
}
