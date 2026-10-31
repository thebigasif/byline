//! What the visitor log says about crawling: how each search and AI
//! crawler reaches the site (GEO C3) and which addresses answered bots
//! with an error (C5). A bot whose address is not on its company's list is
//! never counted as that bot.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};

use crate::{BotCheck, CrawledPage, Crawler, CrawlerKind, Visit, Visitor};

/// How the site answered one kind of visit, counted by status class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Answers {
    /// 2xx and 304: the page.
    pub ok: u64,
    /// Other 3xx: sent elsewhere.
    pub moved: u64,
    /// 4xx: missing or refused.
    pub missing: u64,
    /// 5xx: the server failed.
    pub failed: u64,
}

impl Answers {
    fn add(&mut self, status: u16) {
        let count = match status {
            200..=299 | 304 => &mut self.ok,
            300..=399 => &mut self.moved,
            400..=499 => &mut self.missing,
            _ => &mut self.failed,
        };
        *count = count.saturating_add(1);
    }
}

/// How one search or AI crawler reached the site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlerHealth {
    /// The crawler, as its user agent named it.
    pub crawler: Crawler,
    /// The visits from an address on its company's list.
    pub listed: Answers,
    /// The visits it could not be checked for, having no list.
    pub unchecked: Answers,
    /// The visits from an address not on its list: something else used
    /// its name. Never counted as the crawler.
    pub pretending: u64,
    /// The addresses it fetched, listed or unchecked.
    pub pages: u64,
    /// Its latest visit, listed or unchecked, or its latest listed fetch
    /// kept from before the log's oldest row.
    pub last_at: Option<DateTime<Utc>>,
}

/// Each search and AI crawler in `visits` or in the `kept` pages, search
/// first, then by name.
pub fn crawl_health(visits: &[Visit], kept: &[CrawledPage]) -> Vec<CrawlerHealth> {
    let mut all: Healths<'_> = BTreeMap::new();
    for visit in visits {
        let Visitor::Bot(crawler, check) = visit.visitor else {
            continue;
        };
        if !watched(crawler) {
            continue;
        }
        let (health, pages) = entry(&mut all, crawler);
        match check {
            BotCheck::Listed => health.listed.add(visit.status),
            BotCheck::NoList => health.unchecked.add(visit.status),
            BotCheck::NotListed => {
                health.pretending = health.pretending.saturating_add(1);
                continue;
            }
        }
        pages.insert(visit.path.as_str());
        health.last_at = health.last_at.max(Some(visit.at));
    }
    for page in kept.iter().filter(|page| watched(page.crawler)) {
        let (health, _) = entry(&mut all, page.crawler);
        health.last_at = health.last_at.max(Some(page.last_at));
    }
    all.into_values()
        .map(|(mut health, pages)| {
            health.pages = u64::try_from(pages.len()).unwrap_or(u64::MAX);
            health
        })
        .collect()
}

/// Each crawler's health so far, with the addresses it fetched, search
/// first, then by name.
type Healths<'a> = BTreeMap<(CrawlerKind, &'static str), (CrawlerHealth, BTreeSet<&'a str>)>;

fn entry<'m, 'a>(
    all: &'m mut Healths<'a>,
    crawler: Crawler,
) -> &'m mut (CrawlerHealth, BTreeSet<&'a str>) {
    all.entry((crawler.kind(), crawler.name()))
        .or_insert_with(|| {
            let health = CrawlerHealth {
                crawler,
                listed: Answers::default(),
                unchecked: Answers::default(),
                pretending: 0,
                pages: 0,
                last_at: None,
            };
            (health, BTreeSet::new())
        })
}

/// Whether the crawl reports follow `crawler`: search engines and AI.
fn watched(crawler: Crawler) -> bool {
    matches!(crawler.kind(), CrawlerKind::Search | CrawlerKind::Ai)
}

/// One address that answered search or AI crawlers with one error status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotError {
    /// The address asked for.
    pub path: String,
    /// The status, 400 or more.
    pub status: u16,
    /// The crawlers that got it, by name.
    pub crawlers: Vec<Crawler>,
    /// How many times.
    pub hits: u64,
    /// The latest time.
    pub last_at: DateTime<Utc>,
}

/// The crawlers that got one error, how often, and the latest time.
type ErrorCount = (BTreeSet<Crawler>, u64, DateTime<Utc>);

/// Each address and error status search and AI crawlers got in `visits`,
/// from listed or unchecked addresses, most hits first. Bots whose
/// address is not on their list are left out: they are someone else.
pub fn bot_errors(visits: &[Visit]) -> Vec<BotError> {
    let mut errors: BTreeMap<(&str, u16), ErrorCount> = BTreeMap::new();
    for visit in visits.iter().filter(|visit| visit.status >= 400) {
        let Visitor::Bot(crawler, check) = visit.visitor else {
            continue;
        };
        if check == BotCheck::NotListed || !watched(crawler) {
            continue;
        }
        let (crawlers, hits, last) = errors
            .entry((visit.path.as_str(), visit.status))
            .or_insert_with(|| (BTreeSet::new(), 0, visit.at));
        crawlers.insert(crawler);
        *hits = hits.saturating_add(1);
        *last = (*last).max(visit.at);
    }
    let mut errors: Vec<BotError> = errors
        .into_iter()
        .map(|((path, status), (crawlers, hits, last_at))| BotError {
            path: path.to_owned(),
            status,
            crawlers: crawlers.into_iter().collect(),
            hits,
            last_at,
        })
        .collect();
    errors.sort_by_key(|error| Reverse(error.hits));
    errors
}
