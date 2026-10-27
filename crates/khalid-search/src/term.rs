//! One search term: a word as the index stores it.

use std::fmt;

/// A word in lower case and cut to its stem, as the index stores it and a
/// query looks it up. Only an [`Analyzer`](crate::Analyzer) makes one, so a
/// term and the text it came from always went through the same rules.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Term(String);

impl Term {
    /// The term made by an analyzer from one word.
    pub(crate) fn new(text: String) -> Self {
        Self(text)
    }

    /// The term as text, such as `link` for "Linking".
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
