//! Finds the sections of a rendered page that a static site swaps whole for
//! one note: the comments under a post, and gated content.
//!
//! A section is found by its class, `comments` or `gated`, and ends at the
//! `</section>` that closes it, counting any sections inside it.

use crate::page_form::{attributes, open_at, tag_end};
use crate::{LockError, ServerFeature};

/// What a section a static page cannot keep holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SectionKind {
    /// The comments under a post, with their forms.
    Comments,
    /// Gated content: its note and the sign-up that would open the rest.
    Gated,
}

impl SectionKind {
    /// The feature a section of this kind needs.
    pub(crate) fn feature(self) -> ServerFeature {
        match self {
            Self::Comments => ServerFeature::Comments,
            Self::Gated => ServerFeature::Newsletter,
        }
    }
}

/// One section to lock: where it starts and ends, as byte offsets, and its
/// kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PageSection {
    /// Where its `<section` tag starts.
    pub(crate) start: usize,
    /// Just past the `>` of the `</section>` tag that closes it.
    pub(crate) end: usize,
    /// What it holds.
    pub(crate) kind: SectionKind,
}

/// The tag a section starts with.
const OPEN: &str = "<section";

/// The tag a section ends with, before its `>`.
const CLOSE: &str = "</section";

/// Every section of `html`, a rendered page, that a static site locks, in
/// the order they appear. A locked section's own sections are part of it.
///
/// # Errors
///
/// [`LockError::UnclosedSection`] when one of them never ends.
pub(crate) fn sections(html: &str) -> Result<Vec<PageSection>, LockError> {
    // ASCII lower case keeps every byte where it was, so offsets carry over.
    let lower = html.to_ascii_lowercase();
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(start) = open_at(&lower, at, OPEN) {
        let unclosed = LockError::UnclosedSection { offset: start };
        let after = tag_end(&lower, start + OPEN.len()).ok_or(unclosed)?;
        let tag = lower.get(start + OPEN.len()..after - 1).unwrap_or_default();
        let Some(kind) = kind(&attributes(tag)) else {
            at = after;
            continue;
        };
        let end = closed_at(&lower, after).ok_or(LockError::UnclosedSection { offset: start })?;
        found.push(PageSection { start, end, kind });
        at = end;
    }
    Ok(found)
}

/// The kind of a section with `attributes`, read from its classes, or
/// `None` for a section a static page keeps.
fn kind(attributes: &[(&str, &str)]) -> Option<SectionKind> {
    let classes = attributes
        .iter()
        .find(|&&(name, _)| name == "class")
        .map_or("", |&(_, value)| value);
    let has = |wanted: &str| {
        classes
            .split_ascii_whitespace()
            .any(|class| class == wanted)
    };
    if has("comments") {
        Some(SectionKind::Comments)
    } else if has("gated") {
        Some(SectionKind::Gated)
    } else {
        None
    }
}

/// Just past the `>` of the `</section>` that closes a section whose tag
/// ends at `from`, or `None` when it is never closed.
fn closed_at(lower: &str, from: usize) -> Option<usize> {
    let mut depth = 1_usize;
    let mut at = from;
    loop {
        let close = at + lower.get(at..)?.find(CLOSE)?;
        match open_at(lower, at, OPEN) {
            Some(open) if open < close => {
                depth += 1;
                at = open + OPEN.len();
            }
            _ => {
                let end = close + lower.get(close..)?.find('>')? + 1;
                depth -= 1;
                if depth == 0 {
                    return Some(end);
                }
                at = end;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_is_read_from_its_whole_classes() {
        assert_eq!(kind(&[("class", "comments")]), Some(SectionKind::Comments));
        assert_eq!(kind(&[("class", "post gated")]), Some(SectionKind::Gated));
        assert_eq!(kind(&[("class", "comments-none")]), None);
        assert_eq!(kind(&[("id", "comments")]), None);
    }

    #[test]
    fn a_section_inside_counts_toward_where_it_ends() {
        let html = "<section class=\"comments\"><section>a</section>b</section>c";
        let found = sections(html).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(html.get(found[0].end..), Some("c"));
    }

    #[test]
    fn a_section_left_open_is_an_error() {
        let error = sections("<p>a</p><section class=\"gated\"><p>b</p>").unwrap_err();
        assert!(
            matches!(error, LockError::UnclosedSection { offset: 8 }),
            "{error}"
        );
    }
}
