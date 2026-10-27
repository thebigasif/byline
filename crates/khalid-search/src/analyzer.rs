//! Splitting text into words and turning each word into a term.

use std::fmt;
use std::ops::Range;

use rust_stemmers::{Algorithm, Stemmer};

use crate::{Language, Term};

/// Words longer than this many bytes are left out of the index. Real words
/// are far shorter; anything longer is a pasted hash or a run of code.
const LONGEST_WORD: usize = 60;

/// Turns text into terms with the rules of one language. Indexing and
/// querying use the same analyzer, so a query meets the text on the same
/// terms.
pub struct Analyzer {
    language: Language,
    stemmer: Option<Stemmer>,
}

impl fmt::Debug for Analyzer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Analyzer")
            .field("language", &self.language)
            .finish_non_exhaustive()
    }
}

/// One word found in a text: where it is, and the term it becomes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Word {
    /// The word's bytes in the text it was found in, so a snippet can mark
    /// the word as it was written.
    pub range: Range<usize>,
    /// The term the word becomes.
    pub term: Term,
}

impl Analyzer {
    /// The analyzer for `language`.
    pub fn new(language: Language) -> Self {
        let stemmer = match language {
            Language::English => Some(Stemmer::create(Algorithm::English)),
            Language::German => Some(Stemmer::create(Algorithm::German)),
            Language::Other => None,
        };
        Self { language, stemmer }
    }

    /// The language whose rules this analyzer applies.
    pub fn language(&self) -> Language {
        self.language
    }

    /// Every word of `text` in order, with the term each becomes. A word is
    /// a run of letters and digits, with an apostrophe allowed inside, as in
    /// "don't". Words too long to be real words are left out.
    pub fn words<'text>(&'text self, text: &'text str) -> impl Iterator<Item = Word> + 'text {
        spans(text).filter_map(move |range| {
            let word = text.get(range.clone())?;
            (word.len() <= LONGEST_WORD).then(|| Word {
                term: self.term(word),
                range,
            })
        })
    }

    /// The terms of `text` in order, without where each word was.
    pub fn terms<'text>(&'text self, text: &'text str) -> impl Iterator<Item = Term> + 'text {
        self.words(text).map(|word| word.term)
    }

    /// The term one word becomes: lower case, then cut to its stem when the
    /// language has stemming rules.
    fn term(&self, word: &str) -> Term {
        let lower = word.to_lowercase().replace('\u{2019}', "'");
        match &self.stemmer {
            Some(stemmer) => Term::new(stemmer.stem(&lower).replace('\'', "")),
            None => Term::new(lower.replace('\'', "")),
        }
    }
}

/// `word` in lower case without apostrophes and not cut to its stem: how
/// the start of a word a person is still typing is compared with terms.
pub(crate) fn unstemmed(word: &str) -> String {
    word.to_lowercase().replace(['\'', '\u{2019}'], "")
}

/// Whether `c` belongs inside a word.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

/// Whether `c` is an apostrophe, which stays in a word only between two
/// word characters.
fn is_apostrophe(c: char) -> bool {
    c == '\'' || c == '\u{2019}'
}

/// The byte ranges of the words in `text`.
fn spans(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    let mut chars = text.char_indices().peekable();
    std::iter::from_fn(move || {
        let start = loop {
            let (at, c) = chars.next()?;
            if is_word_char(c) {
                break at;
            }
        };
        let mut end = text.len();
        while let Some(&(at, c)) = chars.peek() {
            if is_word_char(c) {
                chars.next();
                continue;
            }
            let inside = is_apostrophe(c)
                && text
                    .get(at + c.len_utf8()..)
                    .and_then(|rest| rest.chars().next())
                    .is_some_and(is_word_char);
            if inside {
                chars.next();
                continue;
            }
            end = at;
            break;
        }
        Some(start..end)
    })
}
