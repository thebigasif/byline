//! Which way a language's text runs, for the `dir` attribute.

use khalid_core::Locale;
use serde::Serialize;

/// Languages written right to left, by their two- or three-letter code.
const RIGHT_TO_LEFT_LANGUAGES: [&str; 10] =
    ["ar", "ckb", "dv", "fa", "he", "ps", "sd", "ug", "ur", "yi"];

/// Scripts written right to left, by their four-letter code.
const RIGHT_TO_LEFT_SCRIPTS: [&str; 6] = ["Adlm", "Arab", "Hebr", "Nkoo", "Syrc", "Thaa"];

/// The direction text runs in, as the `dir` attribute names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Direction {
    /// Left to right, as in English and German.
    Ltr,
    /// Right to left, as in Arabic and Hebrew.
    Rtl,
}

impl Direction {
    /// The direction of `locale`. A script subtag wins over the language, so
    /// `az-Arab` runs right to left while `az` runs left to right.
    pub(crate) fn of(locale: &Locale) -> Self {
        let tag = locale.as_str();
        let script = tag.split('-').nth(1).filter(|part| part.len() == 4);
        let rtl = match script {
            Some(script) => RIGHT_TO_LEFT_SCRIPTS.contains(&script),
            None => RIGHT_TO_LEFT_LANGUAGES.contains(&locale.language()),
        };
        if rtl { Self::Rtl } else { Self::Ltr }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn direction(tag: &str) -> Direction {
        Direction::of(&Locale::try_new(tag).unwrap())
    }

    #[test]
    fn english_and_german_run_left_to_right() {
        assert_eq!(direction("en"), Direction::Ltr);
        assert_eq!(direction("de-AT"), Direction::Ltr);
    }

    #[test]
    fn arabic_and_hebrew_run_right_to_left() {
        assert_eq!(direction("ar"), Direction::Rtl);
        assert_eq!(direction("he-IL"), Direction::Rtl);
    }

    #[test]
    fn the_script_decides_over_the_language() {
        assert_eq!(direction("az-Arab"), Direction::Rtl);
        assert_eq!(direction("ku-Latn"), Direction::Ltr);
    }
}
