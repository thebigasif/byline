//! The AI search checks on how a page's own text is written (phase GEO,
//! A1, built on the linter's W1 and W2): the answer under a question
//! heading and paragraph length.
//!
//! The linter judges a post's source before it is published. These checks
//! judge the page a crawler reads, so they also cover pages Khalid did not
//! write, such as a theme's own pages or a running server.

use crate::page_checks::Found;
use crate::{Block, Check, Page, Paragraph, Thresholds};

/// How many words of a paragraph a detail quotes.
const QUOTED_WORDS: usize = 8;

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
}

/// The checks every paragraph answers: its length.
fn paragraph_issues(paragraph: &Paragraph, thresholds: Thresholds, found: &mut Found<'_>) {
    let text = paragraph.text();
    let words = text.split_whitespace().count();
    if words > thresholds.most_paragraph_words {
        found.add(
            Check::ParagraphTooLong,
            format!("{words} words: {}", quote(text)),
        );
    }
}

/// The first few words of `text`, with `...` when there are more.
fn quote(text: &str) -> String {
    let mut words: Vec<&str> = text.split_whitespace().take(QUOTED_WORDS).collect();
    if text.split_whitespace().count() > QUOTED_WORDS {
        words.push("...");
    }
    words.join(" ")
}
