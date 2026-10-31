//! The limits the audit's checks measure against.

/// The limits a site may change for the audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Thresholds {
    pub(crate) most_links: usize,
    pub(crate) fewest_answer_words: usize,
    pub(crate) most_answer_words: usize,
    pub(crate) most_paragraph_words: usize,
}

impl Default for Thresholds {
    /// 3,000 links on a page, as Semrush counts. An answer of 40 to 60
    /// words under a question heading (decision D3). At most 225 words in
    /// a paragraph (task W2). The two word counts match the linter's.
    fn default() -> Self {
        Self {
            most_links: 3_000,
            fewest_answer_words: 40,
            most_answer_words: 60,
            most_paragraph_words: 225,
        }
    }
}

impl Thresholds {
    /// The same limits with at most `links` links on a page.
    #[must_use]
    pub fn with_most_links(self, links: usize) -> Self {
        Self {
            most_links: links,
            ..self
        }
    }

    /// The same limits with an answer under a question heading of
    /// `fewest` to `most` words, as `lint.toml` may set them.
    #[must_use]
    pub fn with_answer_words(self, fewest: usize, most: usize) -> Self {
        Self {
            fewest_answer_words: fewest,
            most_answer_words: most,
            ..self
        }
    }

    /// The same limits with at most `most` words in a paragraph, as
    /// `lint.toml` may set it.
    #[must_use]
    pub fn with_paragraph_words(self, most: usize) -> Self {
        Self {
            most_paragraph_words: most,
            ..self
        }
    }
}
