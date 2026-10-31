//! Finds each `<form>` in a rendered page and says which feature it needs.
//!
//! Rendered pages escape `<` in text, code and JSON-LD, so every `<form` the
//! scan meets is a real tag. Forms cannot nest, so the first `</form>` after
//! one closes it.

use crate::{LockError, ServerFeature};

/// What a form on a page sends to the site, judged by its address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormKind {
    /// A comment, to an address with a `comment` or `comments` folder.
    Comments,
    /// A sign-up, to an address with a `subscribe` folder.
    Newsletter,
    /// Any other form to the site, such as `/forms/contact`.
    Forms,
    /// A search, to a `search` folder or with `role="search"`.
    Search,
}

impl FormKind {
    /// The feature a form of this kind needs.
    pub fn feature(self) -> ServerFeature {
        match self {
            Self::Comments => ServerFeature::Comments,
            Self::Newsletter => ServerFeature::Newsletter,
            Self::Forms => ServerFeature::Forms,
            Self::Search => ServerFeature::Search,
        }
    }
}

/// One form on a page: where it starts and ends, as byte offsets, and its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PageForm {
    /// Where its `<form` tag starts.
    pub start: usize,
    /// Just past the `>` of its `</form>` tag.
    pub end: usize,
    /// What it sends to the site, or `None` when it sends to another site
    /// or opens an email, which works on any host.
    pub kind: Option<FormKind>,
}

/// Every form on `html`, a rendered page, in the order they appear.
///
/// # Errors
///
/// [`LockError::Unclosed`] when a form's tag or the form itself never ends,
/// so where it ends is unknown.
pub fn forms(html: &str) -> Result<Vec<PageForm>, LockError> {
    // ASCII lower case keeps every byte where it was, so offsets carry over.
    let lower = html.to_ascii_lowercase();
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(form) = next(&lower, at)? {
        at = form.end;
        found.push(form);
    }
    Ok(found)
}

/// The next form at or after `from` in `lower`, a page in ASCII lower case.
fn next(lower: &str, from: usize) -> Result<Option<PageForm>, LockError> {
    let Some(start) = open_at(lower, from, "<form") else {
        return Ok(None);
    };
    let unclosed = LockError::Unclosed { offset: start };
    let opened = "<form".len();
    let Some(tag_end) = tag_end(lower, start + opened) else {
        return Err(unclosed);
    };
    let close = lower
        .get(tag_end..)
        .and_then(|rest| rest.find("</form"))
        .map(|at| tag_end + at);
    let end = close
        .and_then(|at| lower.get(at..)?.find('>').map(|gt| at + gt + 1))
        .ok_or(unclosed)?;
    let tag = lower.get(start + opened..tag_end - 1).unwrap_or_default();
    Ok(Some(PageForm {
        start,
        end,
        kind: kind(&attributes(tag)),
    }))
}

/// Where the next `open` tag, such as `<form`, starts at or after `from`,
/// skipping longer names such as `<formula`.
pub(crate) fn open_at(lower: &str, from: usize, open: &str) -> Option<usize> {
    let mut at = from;
    loop {
        let found = at + lower.get(at..)?.find(open)?;
        let after = lower.as_bytes().get(found + open.len());
        if matches!(
            after,
            Some(b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' | b'>' | b'/')
        ) {
            return Some(found);
        }
        at = found + 1;
    }
}

/// Just past the `>` that ends the tag whose attributes start at `from`,
/// with a `>` inside quotes left alone.
pub(crate) fn tag_end(lower: &str, from: usize) -> Option<usize> {
    let mut quote = None;
    for (at, byte) in lower.bytes().enumerate().skip(from) {
        match (quote, byte) {
            (None, b'"' | b'\'') => quote = Some(byte),
            (Some(open), _) if open == byte => quote = None,
            (None, b'>') => return Some(at + 1),
            _ => {}
        }
    }
    None
}

/// The attributes of a tag, as names and values, in the order written.
pub(crate) fn attributes(tag: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    let mut rest = tag;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == '/');
        if rest.is_empty() {
            return found;
        }
        let name_end = rest
            .find(|c: char| c.is_ascii_whitespace() || c == '=' || c == '/')
            .unwrap_or(rest.len());
        let (name, after) = rest.split_at_checked(name_end).unwrap_or((rest, ""));
        let after = after.trim_start();
        let (value, after) = match after.strip_prefix('=') {
            Some(value) => split_value(value.trim_start()),
            None => ("", after),
        };
        found.push((name, value));
        rest = after;
    }
}

/// An attribute value at the start of `text`, quoted or not, and what follows.
fn split_value(text: &str) -> (&str, &str) {
    if let Some(quote @ ('"' | '\'')) = text.chars().next() {
        let inner = text.get(1..).unwrap_or_default();
        let close = inner.find(quote).unwrap_or(inner.len());
        let value = inner.get(..close).unwrap_or_default();
        (value, inner.get(close + 1..).unwrap_or_default())
    } else {
        let end = text
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(text.len());
        text.split_at_checked(end).unwrap_or((text, ""))
    }
}

/// What a form with `attributes` needs from a server, or `None` when it
/// sends to another site or opens an email, which works on any host.
fn kind(attributes: &[(&str, &str)]) -> Option<FormKind> {
    let value = |name: &str| {
        attributes
            .iter()
            .find(|&&(found, _)| found == name)
            .map_or("", |&(_, value)| value.trim())
    };
    let action = value("action");
    let elsewhere = ["http://", "https://", "//", "mailto:"];
    if elsewhere.iter().any(|start| action.starts_with(start)) {
        return None;
    }
    let path = action.split(['?', '#']).next().unwrap_or_default();
    let segment = |wanted: &[&str]| path.split('/').any(|part| wanted.contains(&part));
    Some(if segment(&["subscribe"]) {
        FormKind::Newsletter
    } else if segment(&["comment", "comments"]) {
        FormKind::Comments
    } else if segment(&["search"]) || value("role") == "search" {
        FormKind::Search
    } else {
        FormKind::Forms
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_read_quoted_bare_and_empty_values() {
        assert_eq!(
            attributes(" class=\"a b\" action='/x' method=post novalidate /"),
            [
                ("class", "a b"),
                ("action", "/x"),
                ("method", "post"),
                ("novalidate", "")
            ]
        );
    }

    #[test]
    fn a_form_with_no_action_sends_to_its_own_page() {
        assert_eq!(kind(&[]), Some(FormKind::Forms));
        assert_eq!(kind(&[("role", "search")]), Some(FormKind::Search));
    }

    #[test]
    fn a_kind_is_read_from_whole_path_segments() {
        let action = |path| kind(&[("action", path)]);
        assert_eq!(action("/blog/research/"), Some(FormKind::Forms));
        assert_eq!(action("/subscribers-only/"), Some(FormKind::Forms));
        assert_eq!(action("/blog/search/?q=x"), Some(FormKind::Search));
        assert_eq!(action("/comments/new"), Some(FormKind::Comments));
    }

    #[test]
    fn a_longer_tag_name_is_not_a_form() {
        assert_eq!(open_at("<formula>x</formula>", 0, "<form"), None);
    }
}
