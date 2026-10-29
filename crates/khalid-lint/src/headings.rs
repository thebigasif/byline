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

use crate::{Hit, LintPost, Rule, RuleId, Severity, SiteView};

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
    /// The largest share in percent two headings may share before they
    /// read as the same heading.
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
        let body = post.body_text.lines().map(str::trim).collect::<Vec<_>>();
        post.headings
            .iter()
            .filter_map(|heading| {
                let lower = heading.text.to_lowercase();
                let wants_list = self
                    .list_words
                    .iter()
                    .any(|word| lower.contains(word));
                let wants_table = self
                    .table_words
                    .iter()
                    .any(|word| lower.contains(word));
                if !wants_list && !wants_table {
                    return None;
                }
                // The lines from this heading to the next one.
                let start = body
                    .iter()
                    .position(|line| *line == heading.text.trim())?;
                let after: Vec<&&str> = body
                    .iter()
                    .skip(start + 1)
                    .take_while(|line| {
                        !post
                            .headings
                            .iter()
                            .any(|h| h.text.trim() == **line)
                    })
                    .collect();
                let has_list = after
                    .iter()
                    .any(|line| line.starts_with("- ") || line.starts_with("* "));
                let has_table = after.iter().any(|line| line.contains(" | "));
                match (wants_list, wants_table) {
                    (true, false) if !has_list => Some(
                        Hit::about(format!("the heading {:?} promises kinds, but no list follows", heading.text))
                            .suggesting(vec!["turn what follows into a list, one kind to a bullet".to_owned()]),
                    ),
                    (false, true) if !has_table => Some(
                        Hit::about(format!("the heading {:?} promises a comparison, but no table follows", heading.text))
                            .suggesting(vec!["set what follows as a table, one row to a pair".to_owned()]),
                    ),
                    (true, true) if !has_list && !has_table => Some(
                        Hit::about(format!("the heading {:?} promises kinds and a comparison, but neither a list nor a table follows", heading.text))
                            .suggesting(vec!["give the kinds a list and the differences a table".to_owned()]),
                    ),
                    _ => None,
                }
            })
            .collect()
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }
}
