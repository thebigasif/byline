//! The four writing checks (phase GEO, W1 to W4).
//!
//! W1: a heading phrased as a question is followed by a standalone answer
//! of 40 to 60 words, so a reader, and an AI answer engine, gets the
//! answer before any detail. W2: a paragraph much past 225 words buries
//! its ideas; the target is 75 to 225 words, one idea each. W3: hedge
//! words, vague openers and trailing which-chains make an answer harder
//! to trust and to lift; the word lists live in the catalogue so a site
//! can read and change them. W4: a number a reader cannot check, with no
//! source and no year, teaches doubt; the fix is a link or a date.
//!
//! The behaviour follows Steve Toth's and Koray Tugberk Gubur's writing
//! guidance for answer engines, as the GEO phase records. Where their
//! guidance gives no exact word counts, the counts come from the
//! maintainer's brief: 40 to 60 words for the capsule (decision D3),
//! about 225 for a paragraph.

use crate::{Hit, LintPost, Rule, RuleId, Severity, SiteView};

/// A question-style heading ends with a question mark.
const QUESTION_MARK: char = '?';

/// **W1: the answer capsule.**
///
/// A heading phrased as a question must be followed by a standalone
/// answer of `min_answer_words` to `max_answer_words` words (40 to 60 by
/// default, decision D3), so the answer is complete before any detail
/// starts.
#[derive(Debug)]
pub(crate) struct AnswerCapsule {
    pub(crate) min_answer_words: usize,
    pub(crate) max_answer_words: usize,
}

impl Rule for AnswerCapsule {
    fn id(&self) -> RuleId {
        RuleId::new("answer-capsule")
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let mut hits = Vec::new();
        for heading in question_headings(post) {
            let Some(words) = words_after(post, &heading.text) else {
                continue;
            };
            if words < self.min_answer_words || words > self.max_answer_words {
                hits.push(
                    Hit::about(format!(
                        "heading \"{}\" is followed by {words} words, not the {} to {} of a standalone answer",
                        heading.text, self.min_answer_words, self.max_answer_words
                    ))
                    .suggesting(vec![format!(
                        "write the whole answer in one paragraph of {} to {} words under the heading",
                        self.min_answer_words, self.max_answer_words
                    )]),
                );
            }
        }
        hits
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }
}

/// **W2: paragraph length.**
///
/// A paragraph over `max_paragraph_words` words (225 by default) buries
/// more than one idea. The target is 75 to 225 words, one idea each.
#[derive(Debug)]
pub(crate) struct LongParagraph {
    pub(crate) max_paragraph_words: usize,
}

impl Rule for LongParagraph {
    fn id(&self) -> RuleId {
        RuleId::new("long-paragraph")
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        post.body_text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter(|line| !is_heading_line(post, line))
            .filter(|line| count_words(line) > self.max_paragraph_words)
            .map(|line| {
                Hit::about(format!(
                    "paragraph \"{}\" runs {} words, past the {} word target",
                    first_words(line, 8),
                    count_words(line),
                    self.max_paragraph_words
                ))
                .suggesting(vec![format!(
                    "split it at each change of idea, into paragraphs of 75 to {} words",
                    self.max_paragraph_words
                )])
            })
            .collect()
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }
}

/// The plain words of one line, split on whitespace.
fn count_words(line: &str) -> usize {
    line.split_whitespace().count()
}

/// The first `n` words of `line`, with an ellipsis when it holds more.
fn first_words(line: &str, n: usize) -> String {
    let mut words: Vec<&str> = line.split_whitespace().take(n).collect();
    if line.split_whitespace().count() > n {
        words.push("...");
    }
    words.join(" ")
}

/// The question-style headings of `post`, in body order.
fn question_headings(post: &LintPost) -> Vec<&crate::Heading> {
    post.headings
        .iter()
        .filter(|heading| heading.text.ends_with(QUESTION_MARK))
        .collect()
}

/// How many words follow the heading `text` before the next heading, when
/// the post's plain body names them. `None` when the heading's line
/// cannot be found, so the rule stays quiet rather than guessing.
fn words_after(post: &LintPost, text: &str) -> Option<usize> {
    let mut lines = post.body_text.lines();
    let start = lines.position(|line| line.trim() == text)?;
    Some(
        post.body_text
            .lines()
            .skip(start + 1)
            .take_while(|line| !is_heading_line(post, line))
            .map(count_words)
            .sum(),
    )
}

/// Whether `line` is one of the post's headings, by its text.
fn is_heading_line(post: &LintPost, line: &str) -> bool {
    post.headings
        .iter()
        .any(|heading| heading.text == line.trim())
}

/// The hedge words, vague openers and vague closers W3 flags, in
/// lowercase, kept here so a site can read and change them. Each word is
/// one a reader can delete without losing meaning.
pub(crate) const HEDGE_WORDS: &[&str] = &[
    "arguably",
    "perhaps",
    "possibly",
    "seemingly",
    "somewhat",
    "basically",
    "generally",
    "usually",
    "probably",
    "maybe",
    "kind of",
    "sort of",
    "fairly",
    "quite",
    "rather",
];

/// Openers that promise nothing specific, in lowercase, at the start of a
/// sentence.
pub(crate) const VAGUE_OPENERS: &[&str] = &[
    "there are",
    "there is",
    "it is important",
    "it is worth",
    "it should be noted",
    "needless to say",
    "as we all know",
    "everyone knows",
];

/// **W3: plain sentences in answer paragraphs.**
///
/// Hedge words, vague openers and trailing which-chains are flagged in
/// the paragraphs that answer a question-style heading, where an answer
/// engine lifts the text.
#[derive(Debug)]
pub(crate) struct PlainAnswer {
    pub(crate) hedges: Vec<&'static str>,
    pub(crate) openers: Vec<&'static str>,
}

impl Rule for PlainAnswer {
    fn id(&self) -> RuleId {
        RuleId::new("plain-answer")
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let mut hits = Vec::new();
        for heading in question_headings(post) {
            for line in answer_paragraphs(post, &heading.text) {
                let lower = line.to_lowercase();
                let mut found: Vec<&str> = self
                    .hedges
                    .iter()
                    .copied()
                    .filter(|word| lower.contains(word))
                    .collect();
                found.extend(
                    self.openers
                        .iter()
                        .copied()
                        .filter(|opener| lower.starts_with(opener)),
                );
                if lower.ends_with("which") {
                    found.push("which");
                }
                if !found.is_empty() {
                    hits.push(
                        Hit::about(format!(
                            "the answer under \"{}\" uses {}",
                            heading.text,
                            found.join(", ")
                        ))
                        .suggesting(vec!["say the claim outright and drop the hedge".to_owned()]),
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

/// The paragraph lines that answer the heading `text`, until the next
/// heading.
fn answer_paragraphs<'a>(post: &'a LintPost, text: &str) -> Vec<&'a str> {
    let Some(start) = post.body_text.lines().position(|line| line.trim() == text) else {
        return Vec::new();
    };
    post.body_text
        .lines()
        .skip(start + 1)
        .take_while(|line| !is_heading_line(post, line))
        .filter(|line| !line.trim().is_empty())
        .collect()
}

/// **W4: sourced statistics.**
///
/// A number a reader cannot check, with no linked source near it and no
/// year beside it, is flagged.
#[derive(Debug)]
pub(crate) struct SourcedStatistics;

impl Rule for SourcedStatistics {
    fn id(&self) -> RuleId {
        RuleId::new("sourced-statistics")
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let linked = linked_lines(post);
        post.body_text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter(|line| !is_heading_line(post, line))
            .filter(|line| !linked.iter().any(|linked| linked == line))
            .filter(|line| bare_number(line))
            .map(|line| {
                Hit::about(first_words(line, 8)).suggesting(vec![
                    "link the number to its source, or name the year it held".to_owned(),
                ])
            })
            .collect()
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }
}

/// Whether `line` holds a count or share with no year beside it.
fn bare_number(line: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    for (index, word) in words.iter().enumerate() {
        if !word.chars().any(|c: char| c.is_ascii_digit()) {
            continue;
        }
        let near_year = words
            .iter()
            .skip(index.saturating_sub(2))
            .take(5)
            .any(is_year);
        if !near_year {
            return true;
        }
    }
    false
}

/// Whether `word` is a four-digit year a statistic can hang on, from 1900
/// on, optionally with punctuation around it.
fn is_year(word: &&str) -> bool {
    let bare = word.trim_matches(|c: char| !c.is_ascii_digit());
    (1900..=2200).contains(&bare.parse::<u16>().unwrap_or(0))
}

/// The lines of `post`'s body that carry a link, so a number in them is
/// treated as sourced.
fn linked_lines(post: &LintPost) -> Vec<&str> {
    post.body_text
        .lines()
        .filter(|line| {
            post.links
                .iter()
                .any(|link| !link.text.is_empty() && line.contains(link.text.as_str()))
        })
        .collect()
}
