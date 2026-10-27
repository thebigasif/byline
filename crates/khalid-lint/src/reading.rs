//! How hard a post is to read, measured with LIX.
//!
//! LIX, the Swedish readability index, adds the average sentence length in
//! words to the share of words longer than six letters, in percent. It
//! needs no syllable counts, so it reads English and German alike: below 40
//! is easy, 50 is a newspaper, and 60 or more is very hard in both. Only the
//! sentence ends differ by language, because each language has its own short
//! forms with a full stop, such as "e.g." or "z. B.".

use crate::{Hit, LintPost, Rule, RuleId, Severity, SiteView, german};

/// Short forms that end with a full stop inside a sentence, by language,
/// in lowercase.
const ABBREVIATIONS: &[(&str, &[&str])] = &[
    (
        "en",
        &[
            "e.g.", "i.e.", "etc.", "vs.", "mr.", "mrs.", "ms.", "dr.", "no.", "approx.",
        ],
    ),
    ("de", german::ABBREVIATIONS),
];

/// A word longer than this many letters counts as long, as LIX defines it.
const LONG_WORD_LETTERS: usize = 6;

/// The fewest words before the index means anything; shorter posts pass.
const MIN_WORDS: usize = 100;

/// The words, long words and sentences of a text.
#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    words: usize,
    long_words: usize,
    sentences: usize,
}

impl Counts {
    /// The LIX of these counts, rounded down, or `None` with no words.
    fn index(&self) -> Option<usize> {
        let per_sentence = self.words.checked_div(self.sentences.max(1))?;
        let long_share = self
            .long_words
            .saturating_mul(100)
            .checked_div(self.words)?;
        Some(per_sentence.saturating_add(long_share))
    }
}

/// Counts `text` in `language`. A sentence ends at `.`, `!` or `?` after a
/// word that is not a short form, and at a line break, which ends a
/// heading, a list item or a paragraph without a full stop.
fn count(text: &str, language: &str) -> Counts {
    let short_forms = ABBREVIATIONS
        .iter()
        .find(|(lang, _)| *lang == language)
        .map_or(&[][..], |(_, forms)| *forms);
    let mut counts = Counts::default();
    for line in text.lines() {
        let mut open = false;
        for token in line.split_whitespace() {
            let letters = token.chars().filter(|c| c.is_alphabetic()).count();
            if letters > 0 {
                counts.words += 1;
                open = true;
            }
            if letters > LONG_WORD_LETTERS {
                counts.long_words += 1;
            }
            let bare = token.trim_end_matches(['"', '\'', ')', '\u{201d}', '\u{2019}']);
            let ends = bare.ends_with(['.', '!', '?']);
            if ends && open && !short_forms.contains(&bare.to_lowercase().as_str()) {
                counts.sentences += 1;
                open = false;
            }
        }
        if open {
            counts.sentences += 1;
        }
    }
    counts
}

/// A post of at least 100 words whose LIX is above the site's limit.
#[derive(Debug)]
pub(crate) struct HardToRead {
    /// The highest index allowed, from `Thresholds::hard_reading_index`.
    pub(crate) max: usize,
}

impl Rule for HardToRead {
    fn id(&self) -> RuleId {
        RuleId::new("hard-to-read")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let counts = count(&post.body_text, post.locale.language());
        if counts.words < MIN_WORDS {
            return Vec::new();
        }
        counts
            .index()
            .filter(|index| *index > self.max)
            .map(|index| Hit::about(format!("reading index {index}")))
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_forms_do_not_end_a_sentence_in_their_language() {
        let en = count("Use plain words, e.g. short ones. Then stop.", "en");
        assert_eq!(en.sentences, 2);
        let de = count("Kurze Wörter, z. B. diese, helfen. Fertig.", "de");
        assert_eq!(de.sentences, 2);
        let other = count("Kurze Wörter, z. B. diese.", "en");
        assert_eq!(other.sentences, 3);
    }

    #[test]
    fn a_line_break_ends_a_heading_without_a_full_stop() {
        let counts = count("Why links matter\nLinks help readers.\n", "en");
        assert_eq!(counts.sentences, 2);
        assert_eq!(counts.words, 6);
    }

    #[test]
    fn the_index_adds_sentence_length_and_long_word_share() {
        let counts = Counts {
            words: 100,
            long_words: 40,
            sentences: 4,
        };
        assert_eq!(counts.index(), Some(65));
        assert_eq!(Counts::default().index(), None);
    }
}
