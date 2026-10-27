//! A page's text as the index stores it: each term, counted per field.

use std::collections::BTreeMap;

use crate::{Analyzer, FieldCounts, Term};

/// What the index stores for one page: how many words each field has, and
/// how often each term appears in each field.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexedText {
    lengths: FieldCounts,
    terms: BTreeMap<Term, FieldCounts>,
}

impl IndexedText {
    /// How many words the title, the headings and the body have.
    pub fn lengths(&self) -> FieldCounts {
        self.lengths
    }

    /// Every term of the page, in order, with how often it appears in each
    /// field.
    pub fn terms(&self) -> &BTreeMap<Term, FieldCounts> {
        &self.terms
    }
}

/// Which field a word was found in.
#[derive(Debug, Clone, Copy)]
enum Field {
    Title,
    Headings,
    Body,
}

impl Field {
    /// One word in this field, as counts.
    fn one(self) -> FieldCounts {
        match self {
            Self::Title => FieldCounts::new(1, 0, 0),
            Self::Headings => FieldCounts::new(0, 1, 0),
            Self::Body => FieldCounts::new(0, 0, 1),
        }
    }
}

impl Analyzer {
    /// A page's `title`, `headings` and `body` as the index stores them.
    /// The headings may come as one text, a line each. Leave the headings
    /// out of the body, or they count twice.
    pub fn index(&self, title: &str, headings: &str, body: &str) -> IndexedText {
        let mut text = IndexedText::default();
        let fields = [
            (Field::Title, title),
            (Field::Headings, headings),
            (Field::Body, body),
        ];
        for (field, words) in fields {
            for term in self.terms(words) {
                let one = field.one();
                text.lengths = text.lengths.plus(one);
                let counts = text.terms.entry(term).or_default();
                *counts = counts.plus(one);
            }
        }
        text
    }
}
