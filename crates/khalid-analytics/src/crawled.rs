//! The pages a checked bot fetched, kept after the visitor log erases its
//! rows, so the first crawl of a post is known for good (GEO C2).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::{BotCheck, Crawler, Visit, Visitor};

/// One page one bot fetched from an address on its company's list, with
/// the first and the last time it was answered with the page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawledPage {
    /// The address fetched, such as `/blog/geo/`.
    pub path: String,
    /// The bot, checked against its company's list.
    pub crawler: Crawler,
    /// The first fetch the site saw.
    pub first_at: DateTime<Utc>,
    /// The latest fetch the site saw.
    pub last_at: DateTime<Utc>,
}

impl CrawledPage {
    /// The pages in `visits` fetched by a listed bot and answered with the
    /// page, a 2xx or a 304, one per page and bot, by page then bot. A
    /// bot whose address is not on a list, or that has no list, is left
    /// out: anyone can claim its name.
    pub fn from_visits(visits: &[Visit]) -> Vec<Self> {
        let mut pages: BTreeMap<(&str, Crawler), Span> = BTreeMap::new();
        for visit in visits {
            let Visitor::Bot(crawler, BotCheck::Listed) = visit.visitor else {
                continue;
            };
            if !answered_with_page(visit.status) {
                continue;
            }
            pages
                .entry((visit.path.as_str(), crawler))
                .and_modify(|(first, last)| {
                    *first = (*first).min(visit.at);
                    *last = (*last).max(visit.at);
                })
                .or_insert((visit.at, visit.at));
        }
        pages
            .into_iter()
            .map(|((path, crawler), (first_at, last_at))| Self {
                path: path.to_owned(),
                crawler,
                first_at,
                last_at,
            })
            .collect()
    }
}

/// The first and the last fetch.
type Span = (DateTime<Utc>, DateTime<Utc>);

/// Whether `status` handed the bot the page: a 2xx, or a 304 that says
/// the copy it holds is current.
fn answered_with_page(status: u16) -> bool {
    (200..300).contains(&status) || status == 304
}
