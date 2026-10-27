//! What a reader searched for, as terms.

use crate::analyzer::unstemmed;
use crate::{Analyzer, Term};

/// A query uses at most this many words; the rest are left out. Eight
/// words find any page a person means, and each word costs one lookup.
const MOST_TERMS: usize = 8;

/// How the last word of a query is matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Completion {
    /// Every word is matched whole, as when a person submits the form.
    WholeWords,
    /// The last word may also be the start of a longer word, as while a
    /// person is still typing it.
    LastWordStarts,
}

/// What a reader searched for: each distinct term once, in the order typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    terms: Vec<QueryTerm>,
}

/// One word of a query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryTerm {
    term: Term,
    start: Option<String>,
}

impl Query {
    /// The query in `text`, with the rules of `analyzer`. `None` when the
    /// text has no word at all.
    pub fn parse(analyzer: &Analyzer, text: &str, completion: Completion) -> Option<Self> {
        let mut terms: Vec<QueryTerm> = Vec::new();
        // The term the last word became, and the word as typed.
        let mut last = None;
        for word in analyzer.words(text) {
            let typed = text.get(word.range.clone());
            let at = match terms.iter().position(|known| known.term == word.term) {
                Some(at) => at,
                None if terms.len() == MOST_TERMS => break,
                None => {
                    terms.push(QueryTerm {
                        term: word.term,
                        start: None,
                    });
                    terms.len() - 1
                }
            };
            last = typed.map(|typed| (at, typed));
        }
        if completion == Completion::LastWordStarts
            && let Some((at, typed)) = last
            && let Some(term) = terms.get_mut(at)
        {
            term.start = Some(unstemmed(typed));
        }
        (!terms.is_empty()).then_some(Self { terms })
    }

    /// The terms, each once, in the order typed.
    pub fn terms(&self) -> &[QueryTerm] {
        &self.terms
    }
}

impl QueryTerm {
    /// The term the word becomes.
    pub fn term(&self) -> &Term {
        &self.term
    }

    /// The word as typed, in lower case, when any term that starts with it
    /// also matches. Only the last word of a query typed with
    /// [`Completion::LastWordStarts`] has one.
    pub fn start(&self) -> Option<&str> {
        self.start.as_deref()
    }

    /// Whether the index term `candidate` matches this word: it is the
    /// word's term, or it begins with the word as typed.
    pub fn matches(&self, candidate: &str) -> bool {
        candidate == self.term.as_str()
            || self
                .start
                .as_deref()
                .is_some_and(|start| candidate.starts_with(start))
    }
}
