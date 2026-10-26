//! Rules about the words in a post, and the filler words they share.

use std::collections::BTreeMap;

use crate::{Hit, LintPost, Rule, RuleId, Severity, SiteView, german};

/// Small words that carry no topic, by language. Short on purpose: a word
/// belongs here only if it never names what a post is about.
const STOPWORDS: &[(&str, &[&str])] = &[
    (
        "en",
        &[
            "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "in", "is", "it", "of",
            "on", "or", "that", "the", "this", "to", "was", "with", "you", "your",
        ],
    ),
    ("de", german::STOPWORDS),
];

/// Whether `word`, in lowercase, is a filler word in `language`.
pub(crate) fn is_stopword(language: &str, word: &str) -> bool {
    STOPWORDS
        .iter()
        .filter(|(lang, _)| *lang == language)
        .any(|(_, words)| words.contains(&word))
}

/// The fewest repeats before a phrase can count as stuffing, so a short
/// post where every phrase is a large share is not flagged.
const MIN_REPEATS: usize = 3;

/// One phrase of two or three words filling too large a share of the post.
#[derive(Debug)]
pub(crate) struct KeywordStuffing {
    /// The largest share in percent, from `Thresholds::keyword_density_percent`.
    pub(crate) max_percent: usize,
}

impl Rule for KeywordStuffing {
    fn id(&self) -> RuleId {
        RuleId::new("keyword-stuffing")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let language = post.locale.language();
        let lower = post.body_text.to_lowercase();
        let words: Vec<&str> = lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .collect();
        let mut repeats: BTreeMap<String, usize> = BTreeMap::new();
        for window in words.windows(2).chain(words.windows(3)) {
            let (Some(first), Some(last)) = (window.first(), window.last()) else {
                continue;
            };
            if !is_stopword(language, first) && !is_stopword(language, last) {
                *repeats.entry(window.join(" ")).or_default() += 1;
            }
        }
        // A phrase's share of the post: its repeats times its length in words.
        let worst = repeats
            .into_iter()
            .filter(|&(_, count)| count >= MIN_REPEATS)
            .map(|(phrase, count)| (count * phrase.split(' ').count(), phrase))
            .filter(|&(filled, _)| filled * 100 > self.max_percent * words.len())
            .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
        worst
            .map(|(_, phrase)| Hit::about(phrase))
            .into_iter()
            .collect()
    }
}
