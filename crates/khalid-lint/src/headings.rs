//! The heading and format checks (phase GEO, W5 and W6).
//!
//! W5: the post's title and its first H1 should agree, because a page
//! whose headline and title tag differ reads as two pages, and no two
//! headings in a post should be near-duplicates, because a reader who
//! meets the same heading twice cannot tell the sections apart. W6: a
//! heading that promises types or kinds is answered by a list, and a
//! heading that promises differences, or a comparison page, by a table,
//! because those are the shapes a reader can scan and an engine can
//! lift.
//!
//! The behaviour follows Steve Toth's and Koray Tugberk Gubur's guidance
//! that headings set a promise the body must keep. The exact signals
//! (which words promise a list or a table) are kept here as readable
//! lists, so a site can read and change them.

use crate::{Heading, Hit, LintPost, Rule, RuleId, SectionShape, Severity, SiteView};

/// Heading words that promise a list of types or kinds.
pub(crate) const LIST_WORDS: &[&str] = &[
    "types", "kinds", "examples", "ways", "reasons", "benefits", "steps",
];

/// Heading words that promise a comparison table.
pub(crate) const TABLE_WORDS: &[&str] = &["differences", "vs", "versus", "compared", "comparison"];

/// **W5: the title and the H1 agree, and no two headings are
/// near-duplicates.**
#[derive(Debug)]
pub(crate) struct HeadingAgrees {
    /// The share in percent of the shorter heading's words that, when two
    /// headings share it or more, makes them read as the same heading.
    pub(crate) max_duplicate_percent: usize,
}

impl Rule for HeadingAgrees {
    fn id(&self) -> RuleId {
        RuleId::new("heading-agrees")
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        // The title and the H1 agree by construction here: `khalid`
        // renders the title itself as the page's only H1, and a post
        // that writes its own H1 is stopped by `many-headlines`. What is
        // left for this rule is the second half of W5: no two headings
        // near enough in words that a reader cannot tell the sections
        // apart.
        let mut hits = Vec::new();
        let headings: Vec<&str> = post.headings.iter().map(|h| h.text.as_str()).collect();
        for (index, heading) in headings.iter().enumerate() {
            for other in headings.iter().skip(index + 1) {
                if near_duplicate(heading, other, self.max_duplicate_percent) {
                    hits.push(
                        Hit::about(format!(
                            "two headings say near the same thing: {heading:?} and {other:?}"
                        ))
                        .suggesting(vec![
                            "give each heading its own words, or merge the two sections".to_owned(),
                        ]),
                    );
                }
            }
        }
        hits
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }
}

/// Whether `a` and `b`, in lowercase without punctuation, share at least
/// `max_percent` of the shorter one's words, so they read as the same
/// heading.
fn near_duplicate(a: &str, b: &str, max_percent: usize) -> bool {
    let a_words: Vec<String> = words_of(a);
    let b_words: Vec<String> = words_of(b);
    let (short, long) = if a_words.len() <= b_words.len() {
        (&a_words, &b_words)
    } else {
        (&b_words, &a_words)
    };
    if short.is_empty() {
        return false;
    }
    let fold = |word: &String| -> String { word.strip_suffix('s').unwrap_or(word).to_owned() };
    let long_folded: Vec<String> = long.iter().map(&fold).collect();
    let shared = short
        .iter()
        .map(&fold)
        .filter(|word| long_folded.contains(word))
        .count();
    shared.saturating_mul(100) / short.len() >= max_percent
}

/// The lowercase words of `text` without punctuation.
fn words_of(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

/// **W6: a heading about types or kinds is followed by a list; a heading
/// about differences, or a comparison page, by a table.**
#[derive(Debug)]
pub(crate) struct HeadingShape {
    /// The words that promise a list.
    pub(crate) list_words: Vec<&'static str>,
    /// The words that promise a table.
    pub(crate) table_words: Vec<&'static str>,
}

impl Rule for HeadingShape {
    fn id(&self) -> RuleId {
        RuleId::new("heading-shape")
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        post.headings
            .iter()
            .filter_map(|heading| self.missing_shape(heading))
            .collect()
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }
}

impl HeadingShape {
    /// The finding for `heading` when its words promise a list or a table
    /// and its section holds neither of what it promises. Whole words
    /// count, so "always" promises no "ways".
    fn missing_shape(&self, heading: &Heading) -> Option<Hit> {
        let words = words_of(&heading.text);
        let promises = |list: &[&str]| words.iter().any(|word| list.contains(&word.as_str()));
        let wants_list = promises(&self.list_words);
        let wants_table = promises(&self.table_words);
        let has_list = heading.holds.contains(&SectionShape::List);
        let has_table = heading.holds.contains(&SectionShape::Table);
        let text = &heading.text;
        let (about, fix) = match (wants_list, wants_table) {
            (true, false) if !has_list => (
                format!("the heading {text:?} promises kinds, but no list follows"),
                "turn what follows into a list, one kind to a bullet",
            ),
            (false, true) if !has_table => (
                format!("the heading {text:?} promises a comparison, but no table follows"),
                "set what follows as a table, one row to a pair",
            ),
            (true, true) if !has_list && !has_table => (
                format!(
                    "the heading {text:?} promises kinds and a comparison, \
                     but neither a list nor a table follows"
                ),
                "give the kinds a list and the differences a table",
            ),
            _ => return None,
        };
        Some(Hit::about(about).suggesting(vec![fix.to_owned()]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HeadingLevel;

    fn rule() -> HeadingShape {
        HeadingShape {
            list_words: LIST_WORDS.to_vec(),
            table_words: TABLE_WORDS.to_vec(),
        }
    }

    fn heading(text: &str, holds: Vec<SectionShape>) -> Heading {
        Heading::new(HeadingLevel::H2, text).holding(holds)
    }

    #[test]
    fn a_promised_list_that_follows_is_quiet() {
        let types = heading("Types of links", vec![SectionShape::List]);
        assert!(rule().missing_shape(&types).is_none());
    }

    #[test]
    fn a_promised_table_that_is_missing_is_found() {
        let versus = heading("Links vs buttons", vec![SectionShape::List]);
        assert!(rule().missing_shape(&versus).is_some());
    }

    #[test]
    fn only_whole_words_make_a_promise() {
        let always = heading("Why it always matters", Vec::new());
        assert!(rule().missing_shape(&always).is_none());
    }
}
