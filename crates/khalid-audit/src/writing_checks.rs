//! The AI search checks on how a page's own text is written (phase GEO,
//! A1, built on the linter's W1 to W4): the answer under a question
//! heading, paragraph length, plain answers and sourced numbers.
//!
//! The linter judges a post's source before it is published. These checks
//! judge the page a crawler reads, so they also cover pages Khalid did not
//! write, such as a theme's own pages or a running server.

use crate::page_checks::Found;
use crate::{Block, Check, Page, Paragraph, Thresholds};

/// How many words of a paragraph a detail quotes.
const QUOTED_WORDS: usize = 8;

/// Hedge words, in lowercase, that a reader can delete without losing
/// meaning. The same list as the linter's W3 rule.
const HEDGES: &[&str] = &[
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

/// Sentence openers, in lowercase, that promise nothing specific. The same
/// list as the linter's W3 rule.
const VAGUE_OPENERS: &[&str] = &[
    "there are",
    "there is",
    "it is important",
    "it is worth",
    "it should be noted",
    "needless to say",
    "as we all know",
    "everyone knows",
];

/// Runs the writing checks on one page that is an article, or does not
/// say what it is. A list, a profile or the home page holds counts and
/// links rather than answers, as its `og:type` says.
pub(crate) fn writing_issues(page: &Page, thresholds: Thresholds, found: &mut Found<'_>) {
    if page
        .social()
        .get("og:type")
        .is_some_and(|kind| kind != "article")
    {
        return;
    }
    let blocks = page.outline();
    for (at, block) in blocks.iter().enumerate() {
        match block {
            Block::Heading(heading) if heading.trim_end().ends_with('?') => {
                answers(
                    heading,
                    blocks.get(at.saturating_add(1)..),
                    thresholds,
                    found,
                );
            }
            Block::Paragraph(paragraph) => paragraph_issues(paragraph, thresholds, found),
            _ => {}
        }
    }
}

/// The answer under the question `heading`, from the blocks after it.
fn answers(heading: &str, after: Option<&[Block]>, thresholds: Thresholds, found: &mut Found<'_>) {
    let answer: Vec<&Paragraph> = after
        .unwrap_or_default()
        .iter()
        .map_while(|block| match block {
            Block::Paragraph(paragraph) => Some(paragraph),
            _ => None,
        })
        .collect();
    // Every paragraph up to the next heading, as the linter's W1 rule
    // counts it, so a post the linter passes passes here too.
    let words: usize = answer
        .iter()
        .map(|paragraph| paragraph.text().split_whitespace().count())
        .sum();
    if !(thresholds.fewest_answer_words..=thresholds.most_answer_words).contains(&words) {
        found.add(
            Check::AnswerCapsule,
            format!("{words} words under \"{heading}\""),
        );
    }
    for paragraph in answer {
        let unsure = hedges(paragraph.text());
        if !unsure.is_empty() {
            found.add(
                Check::HedgedAnswer,
                format!("\"{heading}\": {}", unsure.join(", ")),
            );
        }
    }
}

/// The checks every paragraph answers: its length and its numbers.
fn paragraph_issues(paragraph: &Paragraph, thresholds: Thresholds, found: &mut Found<'_>) {
    let text = paragraph.text();
    let words = text.split_whitespace().count();
    if words > thresholds.most_paragraph_words {
        found.add(
            Check::ParagraphTooLong,
            format!("{words} words: {}", quote(text)),
        );
    }
    if !paragraph.linked() && bare_number(text) {
        found.add(Check::UnsourcedNumber, quote(text));
    }
}

/// The hedge words, vague openers and chains of `which` in `text`, in that
/// order. A word counts only whole, so "quiet" holds no "quite".
fn hedges(text: &str) -> Vec<&'static str> {
    let mut found: Vec<&'static str> = Vec::new();
    let all = plain_words(text);
    found.extend(HEDGES.iter().filter(|hedge| has_phrase(&all, hedge)));
    for sentence in text.split(['.', '!', '?']) {
        let words = plain_words(sentence);
        found.extend(
            VAGUE_OPENERS
                .iter()
                .filter(|opener| words.starts_with(&format!(" {opener} "))),
        );
        if sentence.to_lowercase().matches(", which").count() > 1 {
            found.push("which ... which");
        }
    }
    found
}

/// `text` in lowercase as its words with one space around each, so a
/// phrase is found only whole.
fn plain_words(text: &str) -> String {
    let words: Vec<String> = text
        .split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect();
    format!(" {} ", words.join(" "))
}

/// Whether `words`, made by [`plain_words`], holds `phrase` whole.
fn has_phrase(words: &str, phrase: &str) -> bool {
    words.contains(&format!(" {phrase} "))
}

/// Whether `text` holds a number with no year within two words of it, as
/// the linter's W4 rule reads it.
fn bare_number(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    words.iter().enumerate().any(|(at, word)| {
        word.chars().any(|c| c.is_ascii_digit())
            && !words
                .iter()
                .skip(at.saturating_sub(2))
                .take(5)
                .any(|near| is_year(near))
    })
}

/// Whether `word` is a year a number can hang on, from 1900 to 2200, with
/// any punctuation around it.
fn is_year(word: &str) -> bool {
    let bare = word.trim_matches(|c: char| !c.is_ascii_digit());
    bare.len() == 4 && (1900..=2200).contains(&bare.parse::<u16>().unwrap_or(0))
}

/// The first few words of `text`, with `...` when there are more.
fn quote(text: &str) -> String {
    let mut words: Vec<&str> = text.split_whitespace().take(QUOTED_WORDS).collect();
    if text.split_whitespace().count() > QUOTED_WORDS {
        words.push("...");
    }
    words.join(" ")
}
