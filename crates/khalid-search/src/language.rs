//! Which rules turn a word into a term.

use khalid_core::Locale;

/// The language whose word endings search knows, chosen from a locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Language {
    /// English: words are cut to their stem, so "links" finds "linking".
    English,
    /// German: words are cut to their stem, so "Häuser" finds "Haus".
    German,
    /// Any other language: words are matched whole, in lower case.
    Other,
}

impl Language {
    /// The language of `locale`, by its first part: `en-GB` is English,
    /// `de-AT` is German.
    pub fn of(locale: &Locale) -> Self {
        match locale.language() {
            "en" => Self::English,
            "de" => Self::German,
            _ => Self::Other,
        }
    }
}
