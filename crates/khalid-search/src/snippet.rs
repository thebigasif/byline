//! A short stretch of a page's text around what a query matched.

use std::ops::Range;

use crate::{Analyzer, Query};

/// How many words a snippet shows: about two lines under a result's title.
const SNIPPET_WORDS: usize = 30;

/// How many words a snippet shows before the first match it is built
/// around, so the match reads in its sentence.
const WORDS_BEFORE: usize = 3;

/// What shows where a snippet cuts the text.
const CUT: &str = "\u{2026}";

/// The stretch of a page's text a result shows, with the words that
/// matched the query marked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snippet {
    parts: Vec<SnippetPart>,
}

/// One run of a snippet's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnippetPart {
    /// Text that did not match, shown as it is.
    Text(String),
    /// One word that matched the query, as it was written.
    Match(String),
}

impl Snippet {
    /// The snippet of `text` for `query`: about thirty words around the
    /// stretch with the most matches, or the opening words when nothing
    /// matches. A cut at either end shows as an ellipsis. Words are
    /// matched with the rules of `analyzer`, as the query was.
    pub fn of(analyzer: &Analyzer, query: &Query, text: &str) -> Self {
        let words: Vec<(Range<usize>, bool)> = analyzer
            .words(text)
            .map(|word| {
                let matched = query
                    .terms()
                    .iter()
                    .any(|wanted| wanted.matches(word.term.as_str()));
                (word.range, matched)
            })
            .collect();
        let (first, last) = window(&words);
        let (Some((start, _)), Some((end, _))) = (words.get(first), words.get(last)) else {
            return Self::default();
        };
        let mut snippet = Self::default();
        let from = if first == 0 {
            0
        } else {
            snippet.text(&format!("{CUT} "));
            start.start
        };
        let to = if last + 1 == words.len() {
            text.len()
        } else {
            end.end
        };
        let mut at = from;
        for (range, _) in words
            .get(first..=last)
            .unwrap_or_default()
            .iter()
            .filter(|(_, matched)| *matched)
        {
            snippet.text(text.get(at..range.start).unwrap_or_default());
            if let Some(word) = text.get(range.clone()) {
                snippet.parts.push(SnippetPart::Match(word.to_owned()));
            }
            at = range.end;
        }
        snippet.text(text.get(at..to).unwrap_or_default());
        if to < text.len() {
            snippet.text(&format!(" {CUT}"));
        }
        snippet
    }

    /// The runs of text in order, matches and the text between them.
    pub fn parts(&self) -> &[SnippetPart] {
        &self.parts
    }

    /// Adds unmatched text, joined to the run before when that is text too.
    fn text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(SnippetPart::Text(before)) = self.parts.last_mut() {
            before.push_str(text);
        } else {
            self.parts.push(SnippetPart::Text(text.to_owned()));
        }
    }
}

/// The first and last word, by index, of the stretch to show: the window of
/// [`SNIPPET_WORDS`] with the most matches, the earliest on a tie, started a
/// few words before its first match.
fn window(words: &[(Range<usize>, bool)]) -> (usize, usize) {
    let count = |from: usize| {
        words
            .iter()
            .skip(from)
            .take(SNIPPET_WORDS)
            .filter(|(_, matched)| *matched)
            .count()
    };
    let mut best = (0, count(0));
    for from in 1..words.len() {
        let found = count(from);
        if found > best.1 {
            best = (from, found);
        }
    }
    let first_match = words
        .iter()
        .skip(best.0)
        .position(|(_, matched)| *matched)
        .map_or(best.0, |offset| best.0 + offset);
    let first = first_match.saturating_sub(WORDS_BEFORE).max(best.0);
    let last = (first + SNIPPET_WORDS).min(words.len()).saturating_sub(1);
    (first, last)
}
