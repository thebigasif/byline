//! A post's brief: the questions, terms and links the writer planned, each
//! checked against the post as written (GEO P6).

use crate::focus::{holds, words};
use crate::{Hit, LintPost, Rule, RuleId, Severity, SiteView};

/// The entries of `list` that hold a word, without their outer spaces, so
/// a blank entry asks for nothing.
fn entries(list: &[String]) -> impl Iterator<Item = &str> {
    list.iter()
        .map(|entry| entry.trim())
        .filter(|entry| !words(entry).is_empty())
}

/// Whether `heading` asks `question`: every word of the question is in the
/// heading, in any order, so "What internal links are" asks "What are
/// internal links?".
fn asks(heading: &[String], question: &[String]) -> bool {
    question.iter().all(|word| heading.contains(word))
}

/// A question in the brief that no heading of the post asks, one hit per
/// question in the brief's order.
#[derive(Debug)]
pub(crate) struct QuestionUnanswered;

impl Rule for QuestionUnanswered {
    fn id(&self) -> RuleId {
        RuleId::new("brief-question-unanswered")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let Some(brief) = &post.brief else {
            return Vec::new();
        };
        let headings: Vec<Vec<String>> = post.headings.iter().map(|h| words(&h.text)).collect();
        entries(&brief.questions)
            .filter(|question| {
                let question = words(question);
                !headings.iter().any(|heading| asks(heading, &question))
            })
            .map(Hit::about)
            .collect()
    }
}

/// A term in the brief that the post's body and headings never use, one hit
/// per term in the brief's order.
#[derive(Debug)]
pub(crate) struct TermMissing;

impl Rule for TermMissing {
    fn id(&self) -> RuleId {
        RuleId::new("brief-term-missing")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let Some(brief) = &post.brief else {
            return Vec::new();
        };
        let body = words(&post.body_text);
        let headings: Vec<Vec<String>> = post.headings.iter().map(|h| words(&h.text)).collect();
        entries(&brief.terms)
            .filter(|term| {
                let term = words(term);
                !holds(&body, &term) && !headings.iter().any(|heading| holds(heading, &term))
            })
            .map(Hit::about)
            .collect()
    }
}

/// `target` without its `#part` or `?query` and its last slash, so
/// `/blog/a/#why` and `/blog/a` name the same page. The home page `/` keeps
/// its slash.
fn page(target: &str) -> &str {
    let end = target.find(['#', '?']).unwrap_or(target.len());
    let path = target.get(..end).unwrap_or(target);
    match path.strip_suffix('/') {
        Some(rest) if !rest.is_empty() => rest,
        _ => path,
    }
}

/// A page in the brief that the post does not link to, one hit per page in
/// the brief's order.
#[derive(Debug)]
pub(crate) struct LinkMissing;

impl Rule for LinkMissing {
    fn id(&self) -> RuleId {
        RuleId::new("brief-link-missing")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let Some(brief) = &post.brief else {
            return Vec::new();
        };
        brief
            .links
            .iter()
            .map(|link| link.trim())
            .filter(|link| !page(link).is_empty())
            .filter(|link| !post.links.iter().any(|l| page(&l.target) == page(link)))
            .map(Hit::about)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_is_the_same_with_or_without_its_slash_part_or_query() {
        assert_eq!(page("/blog/a/#why"), "/blog/a");
        assert_eq!(page("/blog/a?ref=x"), "/blog/a");
        assert_eq!(page("/blog/a"), "/blog/a");
        assert_eq!(page("/"), "/");
        assert_eq!(page("#top"), "");
    }

    #[test]
    fn a_heading_asks_a_question_in_any_word_order() {
        let heading = words("What internal links are");
        assert!(asks(&heading, &words("What are internal links?")));
        assert!(!asks(&heading, &words("How many internal links?")));
    }
}
