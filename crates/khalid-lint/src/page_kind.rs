//! The kind of page a title asks for (phase GEO, P4).
//!
//! Four kinds of title promise a page of a known shape: `Ahrefs vs Semrush`,
//! `7 Ahrefs alternatives`, `Ahrefs review` and `The 10 best SEO tools`.
//! Koray Tugberk Gubur calls such a repeated search pattern a query
//! template, answered by one document template, so every page of a kind is
//! built the same way. `khalid new --template` writes the starter, and the
//! rules in `page_shape.rs` check the shape. The title words below are this
//! project's choice: neither he nor Steve Toth lists them.

use crate::focus::words;
use crate::german;

/// The words that set two compared things apart, in any language.
const VERSUS: &[&str] = &["vs", "vs.", "versus"];

/// The fewest and most picks a number in a title can promise. A larger
/// number, such as the year in "Best SEO tools 2026", is not a count.
const PICKS: std::ops::RangeInclusive<usize> = 2..=100;

/// Title words that ask for each kind of page, by language.
const KIND_WORDS: &[(&str, Words)] = &[
    (
        "en",
        Words {
            alternatives: &["alternative", "alternatives"],
            review: &["review", "reviewed", "reviews"],
            best_of: &["best", "top"],
        },
    ),
    (
        "de",
        Words {
            alternatives: german::ALTERNATIVES_WORDS,
            review: german::REVIEW_WORDS,
            best_of: german::BEST_OF_WORDS,
        },
    ),
];

/// One language's title words for the kinds a single word marks.
#[derive(Debug)]
struct Words {
    alternatives: &'static [&'static str],
    review: &'static [&'static str],
    best_of: &'static [&'static str],
}

/// The kind of page a title asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageKind {
    /// Two named things side by side, as in `Ahrefs vs Semrush`.
    Comparison {
        /// The thing named before `vs`.
        first: String,
        /// The thing named after `vs`.
        second: String,
    },
    /// The choices beside one thing, as in `7 Ahrefs alternatives`.
    Alternatives {
        /// How many choices the title promises, when it gives a number.
        picks: Option<usize>,
    },
    /// A verdict on one thing, as in `Ahrefs review`.
    Review,
    /// A ranked list of picks, as in `The 10 best SEO tools`.
    BestOf {
        /// How many picks the title promises, when it gives a number.
        picks: Option<usize>,
    },
}

impl PageKind {
    /// The kind of page `title`, written in `language`, asks for, or `None`
    /// for a title of no known kind.
    ///
    /// A title that compares with `vs` is a comparison in every language.
    /// The other kinds are read from English and German words; when a
    /// title holds words of two kinds, alternatives win over a review, and
    /// a review over a best-of list.
    pub fn of_title(language: &str, title: &str) -> Option<Self> {
        if let Some((first, second)) = compared(title) {
            return Some(Self::Comparison { first, second });
        }
        let (_, kinds) = KIND_WORDS.iter().find(|(lang, _)| *lang == language)?;
        let title_words = words(title);
        let has = |list: &[&str]| title_words.iter().any(|word| list.contains(&word.as_str()));
        if has(kinds.alternatives) {
            Some(Self::Alternatives {
                picks: picks(title),
            })
        } else if has(kinds.review) {
            Some(Self::Review)
        } else if has(kinds.best_of) {
            Some(Self::BestOf {
                picks: picks(title),
            })
        } else {
            None
        }
    }
}

/// The two things `title` compares with `vs`: the words just before it,
/// from the last colon, and the words just after it, up to a colon, a
/// question mark, a comma or a dash. `None` when either side is empty.
fn compared(title: &str) -> Option<(String, String)> {
    let tokens: Vec<&str> = title.split_whitespace().collect();
    let at = tokens
        .iter()
        .position(|token| VERSUS.contains(&token.to_lowercase().as_str()))?;
    let (before, rest) = tokens.split_at(at);
    let start = before
        .iter()
        .rposition(|token| token.ends_with(':'))
        .map_or(0, |colon| colon + 1);
    let first = before.get(start..).unwrap_or_default().join(" ");
    let mut second = Vec::new();
    for token in rest.iter().skip(1) {
        if matches!(*token, "-" | "\u{2013}") {
            break;
        }
        let trimmed = token.trim_end_matches([':', '?', ',']);
        second.push(trimmed);
        if trimmed.len() < token.len() {
            break;
        }
    }
    let second = second.join(" ");
    let named = |side: &str| side.chars().any(char::is_alphanumeric);
    (named(&first) && named(&second)).then(|| (first, second.trim().to_owned()))
}

/// The first number in `title` that can count picks, as `7` in
/// `7 Ahrefs alternatives`.
fn picks(title: &str) -> Option<usize> {
    title
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|digits| digits.parse::<usize>().ok())
        .find(|number| PICKS.contains(number))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(language: &str, title: &str) -> Option<PageKind> {
        PageKind::of_title(language, title)
    }

    /// The two sides `title`, in `language`, compares, or empty strings.
    fn sides(language: &str, title: &str) -> (String, String) {
        match kind(language, title) {
            Some(PageKind::Comparison { first, second }) => (first, second),
            _ => (String::new(), String::new()),
        }
    }

    #[test]
    fn a_title_with_vs_compares_the_names_beside_it() {
        let want = |first: &str, second: &str| (first.to_owned(), second.to_owned());
        assert_eq!(sides("en", "Ahrefs vs Semrush"), want("Ahrefs", "Semrush"));
        assert_eq!(
            sides("en", "SEO tools: Ahrefs vs. Semrush, which suits a blog?"),
            want("Ahrefs", "Semrush")
        );
        assert_eq!(
            sides("de", "Ghost versus WordPress - der Vergleich"),
            want("Ghost", "WordPress")
        );
        assert_eq!(
            sides("fr", "Ghost vs WordPress"),
            want("Ghost", "WordPress")
        );
    }

    #[test]
    fn vs_with_nothing_on_one_side_compares_nothing() {
        assert_eq!(kind("en", "Vs: a short history"), None);
        assert_eq!(kind("en", "Ahrefs vs"), None);
    }

    #[test]
    fn single_words_mark_the_other_kinds() {
        assert_eq!(
            kind("en", "7 Ahrefs alternatives for a small blog"),
            Some(PageKind::Alternatives { picks: Some(7) })
        );
        assert_eq!(
            kind("en", "Ahrefs review: worth it?"),
            Some(PageKind::Review)
        );
        assert_eq!(
            kind("en", "The best SEO tools"),
            Some(PageKind::BestOf { picks: None })
        );
        assert_eq!(kind("en", "Why internal links matter"), None);
    }

    #[test]
    fn german_titles_have_their_own_words() {
        assert_eq!(
            kind("de", "Die 5 besten Alternativen zu Ahrefs"),
            Some(PageKind::Alternatives { picks: Some(5) })
        );
        assert_eq!(kind("de", "Ahrefs im Test"), Some(PageKind::Review));
        assert_eq!(
            kind("de", "Die 10 besten SEO-Tools"),
            Some(PageKind::BestOf { picks: Some(10) })
        );
        assert_eq!(kind("fr", "Les meilleurs outils"), None);
    }

    #[test]
    fn a_review_wins_over_best_and_a_year_is_no_count() {
        assert_eq!(
            kind("en", "Ahrefs review: the best SEO tool?"),
            Some(PageKind::Review)
        );
        assert_eq!(
            kind("en", "Best SEO tools in 2026"),
            Some(PageKind::BestOf { picks: None })
        );
        assert_eq!(
            kind("en", "Top 10 SEO tools for 2026"),
            Some(PageKind::BestOf { picks: Some(10) })
        );
    }
}
