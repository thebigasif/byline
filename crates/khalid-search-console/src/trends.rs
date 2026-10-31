//! Winners and losers between two windows, decay over months, and the
//! "Go deeper in `SerpDad`" link a number carries (plan 5.11).

use std::collections::BTreeMap;

use crate::PageNumbers;
use crate::client::encode;

/// Where a "Go deeper in `SerpDad`" link leads.
const SERPDAD: &str = "https://app.serpdad.com/go-deeper";

/// How many months in a row a page's clicks must fall to count as decay.
pub const DECAY_MONTHS: usize = 3;

/// One page's clicks in two windows.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Change {
    /// The page's address, or the query.
    pub key: String,
    /// Clicks in the earlier window.
    pub before: f64,
    /// Clicks in the later window.
    pub now: f64,
}

impl Change {
    /// Clicks gained, or lost when below zero.
    pub fn gained(&self) -> f64 {
        self.now - self.before
    }
}

/// The pages that gained and lost the most clicks between two windows.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Movers {
    /// The biggest gains first.
    pub winners: Vec<Change>,
    /// The biggest losses first.
    pub losers: Vec<Change>,
}

/// The `limit` biggest winners and losers from `before` to `now`. A page
/// in only one window had no clicks in the other. Ties go in address
/// order, so the same numbers always give the same list.
pub fn movers(before: &[PageNumbers], now: &[PageNumbers], limit: usize) -> Movers {
    let mut clicks: BTreeMap<&str, (f64, f64)> = BTreeMap::new();
    for row in before {
        clicks.entry(&row.key).or_default().0 += row.clicks;
    }
    for row in now {
        clicks.entry(&row.key).or_default().1 += row.clicks;
    }
    let changes = clicks.into_iter().map(|(key, (before, now))| Change {
        key: key.to_owned(),
        before,
        now,
    });
    let (mut winners, mut losers): (Vec<Change>, Vec<Change>) = changes
        .filter(|change| change.gained() != 0.0)
        .partition(|change| change.gained() > 0.0);
    winners.sort_by(|a, b| b.gained().total_cmp(&a.gained()));
    losers.sort_by(|a, b| a.gained().total_cmp(&b.gained()));
    winners.truncate(limit);
    losers.truncate(limit);
    Movers { winners, losers }
}

/// A page whose clicks fell every month for [`DECAY_MONTHS`] months.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Decay {
    /// The page's address.
    pub key: String,
    /// Its clicks in each month, oldest first.
    pub clicks: Vec<f64>,
}

/// The pages whose clicks fell in each of the last [`DECAY_MONTHS`]
/// months, from `months` of page numbers, oldest first, the biggest fall
/// first. Fewer than [`DECAY_MONTHS`] + 1 months find none.
pub fn decaying(months: &[Vec<PageNumbers>]) -> Vec<Decay> {
    let Some(last) = months.len().checked_sub(DECAY_MONTHS + 1) else {
        return Vec::new();
    };
    let months = months.get(last..).unwrap_or_default();
    let mut clicks: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
    for (month, rows) in months.iter().enumerate() {
        for row in rows {
            let counts = clicks
                .entry(&row.key)
                .or_insert_with(|| vec![0.0; months.len()]);
            if let Some(count) = counts.get_mut(month) {
                *count += row.clicks;
            }
        }
    }
    let mut found: Vec<Decay> = clicks
        .into_iter()
        .filter(|(_, counts)| {
            counts
                .windows(2)
                .all(|pair| matches!(pair, [a, b] if b < a))
        })
        .map(|(key, clicks)| Decay {
            key: key.to_owned(),
            clicks,
        })
        .collect();
    found.sort_by(|a, b| fall(b).total_cmp(&fall(a)));
    found
}

/// Clicks lost from the first month to the last.
fn fall(decay: &Decay) -> f64 {
    let first = decay.clicks.first().copied().unwrap_or_default();
    first - decay.clicks.last().copied().unwrap_or_default()
}

/// A question a number asks that Khalid cannot answer, and `SerpDad` can.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Question {
    /// Why did this page lose clicks?
    WhyItDropped,
    /// What should this site write next?
    WhatToWriteNext,
}

/// The "Go deeper in `SerpDad`" address for `question` about `page`.
pub fn go_deeper(question: Question, page: &str) -> String {
    let asked = match question {
        Question::WhyItDropped => "why-it-dropped",
        Question::WhatToWriteNext => "what-to-write-next",
    };
    let mut link = format!("{SERPDAD}?question={asked}&page=");
    encode(page, &mut link);
    link.push_str("&from=khalid");
    link
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(key: &str, clicks: f64) -> PageNumbers {
        let body = format!(
            r#"{{"rows": [{{"keys": ["{key}"], "clicks": {clicks}, "impressions": 100.0, "ctr": 0.1, "position": 3.0}}]}}"#
        );
        PageNumbers::read_all(body.as_bytes()).unwrap().remove(0)
    }

    #[test]
    fn winners_and_losers_are_ranked_and_a_page_in_one_window_counts() {
        let before = [row("/a/", 10.0), row("/b/", 50.0), row("/c/", 5.0)];
        let now = [
            row("/a/", 30.0),
            row("/b/", 20.0),
            row("/c/", 5.0),
            row("/d/", 4.0),
        ];
        let found = movers(&before, &now, 5);
        let winners: Vec<&str> = found.winners.iter().map(|c| c.key.as_str()).collect();
        assert_eq!(winners, ["/a/", "/d/"]);
        let loser = found.losers.first().unwrap();
        assert_eq!((loser.key.as_str(), loser.gained()), ("/b/", -30.0));
        assert_eq!(movers(&before, &now, 1).winners.len(), 1);
    }

    #[test]
    fn decay_is_three_falls_in_a_row_and_nothing_less() {
        let months = vec![
            vec![row("/old/", 90.0), row("/bumpy/", 90.0), row("/new/", 0.0)],
            vec![row("/old/", 70.0), row("/bumpy/", 60.0), row("/new/", 9.0)],
            vec![row("/old/", 40.0), row("/bumpy/", 70.0)],
            vec![row("/old/", 10.0), row("/bumpy/", 20.0)],
        ];
        let found = decaying(&months);
        assert_eq!(found.len(), 1);
        let decay = found.first().unwrap();
        assert_eq!(decay.key, "/old/");
        assert_eq!(decay.clicks, [90.0, 70.0, 40.0, 10.0]);
        assert_eq!(decaying(months.get(1..).unwrap()), []);
    }

    #[test]
    fn a_link_to_serpdad_names_the_question_and_the_page() {
        assert_eq!(
            go_deeper(Question::WhyItDropped, "https://www.example.com/blog/a/"),
            "https://app.serpdad.com/go-deeper?question=why-it-dropped\
             &page=https%3A%2F%2Fwww.example.com%2Fblog%2Fa%2F&from=khalid"
        );
    }
}
