//! Collections whose address patterns could give two posts one address.

use crate::{Hit, LintCollection, LintPost, Rule, RuleId, Severity, SiteView};

/// The folders of `collection`'s pattern, with `{collection}` replaced by
/// its name, since that placeholder always stands for it.
fn folders(collection: &LintCollection) -> Vec<String> {
    collection
        .permalink
        .split('/')
        .filter(|folder| !folder.is_empty())
        .map(|folder| folder.replace("{collection}", &collection.name))
        .collect()
}

/// How many digits a placeholder that fills a whole folder takes, if fixed.
fn digits_of(folder: &str) -> Option<usize> {
    match folder {
        "{year}" => Some(4),
        "{month}" | "{day}" => Some(2),
        _ => None,
    }
}

/// Whether the folder `pattern`, with placeholders, can be the plain
/// folder `text`.
fn fills(pattern: &str, text: &str) -> bool {
    if pattern.is_empty() {
        return text.is_empty();
    }
    let Some(rest) = pattern.strip_prefix('{') else {
        let literal = pattern.split('{').next().unwrap_or_default();
        let after = pattern.get(literal.len()..).unwrap_or_default();
        return text
            .strip_prefix(literal)
            .is_some_and(|tail| fills(after, tail));
    };
    let Some((name, after)) = rest.split_once('}') else {
        return false;
    };
    let fixed = |width: usize| {
        let number = text
            .get(..width)
            .is_some_and(|d| d.len() == width && d.bytes().all(|b| b.is_ascii_digit()));
        number && text.get(width..).is_some_and(|tail| fills(after, tail))
    };
    match name {
        "year" => fixed(4),
        "month" | "day" => fixed(2),
        _ => (1..=text.len())
            .filter(|&end| text.is_char_boundary(end))
            .any(|end| text.get(end..).is_some_and(|tail| fills(after, tail))),
    }
}

/// Whether two folders could be the same folder of some address. Two
/// folders with placeholders are taken to meet unless both are fixed-width
/// numbers of different widths, which keeps the check on the side of a
/// warning.
fn meet(a: &str, b: &str) -> bool {
    match (a.contains('{'), b.contains('{')) {
        (false, false) => a == b,
        (false, true) => fills(b, a),
        (true, false) => fills(a, b),
        (true, true) => digits_of(a).zip(digits_of(b)).is_none_or(|(x, y)| x == y),
    }
}

/// Whether some address could belong to both collections.
fn overlap(a: &LintCollection, b: &LintCollection) -> bool {
    let (a, b) = (folders(a), folders(b));
    a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| meet(x, y))
}

/// A post in a collection whose address pattern another collection's could
/// also produce, so two posts with one slug would want one address. Names
/// the other collections, in the site's order.
#[derive(Debug)]
pub(crate) struct OverlappingCollections;

impl Rule for OverlappingCollections {
    fn id(&self) -> RuleId {
        RuleId::new("overlapping-collections")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        let collections = site.collections();
        let Some(own) = post
            .collection
            .as_deref()
            .and_then(|name| collections.iter().find(|c| c.name == name))
        else {
            return Vec::new();
        };
        let others: Vec<&str> = collections
            .iter()
            .filter(|other| other.name != own.name && overlap(own, other))
            .map(|other| other.name.as_str())
            .collect();
        if others.is_empty() {
            return Vec::new();
        }
        vec![Hit::about(others.join(", "))]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_placeholder_fills_only_what_it_can_stand_for() {
        assert!(fills("{year}", "2026"));
        assert!(!fills("{year}", "news"));
        assert!(fills("{year}-{month}", "2026-10"));
        assert!(fills("post-{slug}", "post-links"));
        assert!(!fills("post-{slug}", "post-"));
        assert!(!fills("{month}", "1"));
    }

    #[test]
    fn folders_meet_unless_their_fixed_widths_differ() {
        assert!(meet("guides", "guides"));
        assert!(!meet("guides", "news"));
        assert!(meet("{slug}", "{year}"));
        assert!(!meet("{year}", "{month}"));
    }
}
