//! Redirects Khalid suggests or adds on its own: a 301 when a page's
//! address changes, and the nearest page for an address that answered 404.

use std::collections::{BTreeMap, BTreeSet};

use crate::{Destination, Move, Redirect, RedirectRule, UrlPath};

/// The longest last segment compared letter by letter, so a long address
/// cannot make the comparison slow.
const MAX_COMPARED: usize = 120;

/// A permanent move from each page's old address to its new one, for every
/// key in both `before` and `after` whose address changed, in key order.
///
/// A key is whatever names a page across an edit, such as its file. An old
/// address that a page holds in `after` gets no move, because that page
/// answers there now.
pub fn moved_addresses<K: Ord>(
    before: &BTreeMap<K, UrlPath>,
    after: &BTreeMap<K, UrlPath>,
) -> Vec<RedirectRule> {
    let live: BTreeSet<&UrlPath> = after.values().collect();
    before
        .iter()
        .filter_map(|(key, old)| {
            let new = after.get(key)?;
            if new == old || live.contains(old) {
                return None;
            }
            let to = Destination::Path(new.clone());
            let redirect = Redirect::Move {
                kind: Move::Permanent,
                to,
            };
            Some(RedirectRule::new(old.clone(), redirect))
        })
        .collect()
}

/// The page in `pages` nearest to `missing`, judged by the last part of
/// each address: the most words shared, then the fewest letters to change.
/// `None` when no page shares a word and none is a near spelling, so a
/// suggestion is never a wild guess.
pub fn nearest_page<'p>(missing: &UrlPath, pages: &'p [UrlPath]) -> Option<&'p UrlPath> {
    let wanted = last_segment(missing);
    let wanted_words = words(&wanted);
    let near_enough = (wanted.chars().count() / 4).max(2);
    pages
        .iter()
        .filter(|page| *page != missing)
        .filter_map(|page| {
            let segment = last_segment(page);
            let theirs = words(&segment);
            let shared = wanted_words.iter().filter(|w| theirs.contains(*w)).count();
            let distance = edit_distance(&wanted, &segment);
            (shared > 0 || distance <= near_enough).then_some((shared, distance, page))
        })
        // The first of equally near pages wins, so the answer follows
        // the order `pages` came in.
        .fold(
            None,
            |best: Option<(usize, usize, &UrlPath)>, next| match best {
                Some(kept) if (kept.0, next.1) >= (next.0, kept.1) => Some(kept),
                _ => Some(next),
            },
        )
        .map(|(_, _, page)| page)
}

/// The last non-empty part of `path`, lower-cased and cut to
/// [`MAX_COMPARED`] letters.
fn last_segment(path: &UrlPath) -> String {
    let segment = path.as_str().split('/').rfind(|s| !s.is_empty());
    segment
        .unwrap_or_default()
        .to_lowercase()
        .chars()
        .take(MAX_COMPARED)
        .collect()
}

/// The words of a segment: its runs of letters and digits.
fn words(segment: &str) -> BTreeSet<&str> {
    segment
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}

/// How many letters must be added, removed or changed to turn `a` into `b`.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, left) in a.chars().enumerate() {
        let mut diagonal = i;
        let mut next = vec![i + 1];
        for (j, right) in b.iter().enumerate() {
            let above = row.get(j + 1).copied().unwrap_or(usize::MAX);
            let beside = next.last().copied().unwrap_or(usize::MAX);
            let change = diagonal.saturating_add(usize::from(left != *right));
            next.push(
                change
                    .min(above.saturating_add(1))
                    .min(beside.saturating_add(1)),
            );
            diagonal = above;
        }
        row = next;
    }
    row.last().copied().unwrap_or_default()
}
