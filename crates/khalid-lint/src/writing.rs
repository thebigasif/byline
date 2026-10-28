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
