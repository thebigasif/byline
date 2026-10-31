//! The internal linking report: click depth, links in and out, and an
//! internal link rank for every page.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use khalid_core::AbsoluteUrl;

use crate::Crawl;

/// The share of rank a page passes on through its links, in percent, as
/// in the `PageRank` paper.
const PASSED_PERCENT: u64 = 85;
/// How many rounds the rank is spread over. Twenty settle a blog's graph.
const ROUNDS: usize = 20;
/// The whole rank shared out in each round, so integers keep precision.
const TOTAL: u64 = 1_000_000_000;

/// One page in the internal linking report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageLinks {
    url: AbsoluteUrl,
    depth: Option<u32>,
    inbound: usize,
    outbound: usize,
    external: usize,
    rank: u8,
}

impl PageLinks {
    /// The page's address.
    pub fn url(&self) -> &AbsoluteUrl {
        &self.url
    }

    /// Clicks from home, or `None` when only a sitemap leads to it.
    pub fn depth(&self) -> Option<u32> {
        self.depth
    }

    /// How many other pages of the blog link to it, without `nofollow`.
    pub fn inbound(&self) -> usize {
        self.inbound
    }

    /// How many other pages of the blog it links to.
    pub fn outbound(&self) -> usize {
        self.outbound
    }

    /// How many addresses outside the blog it links to.
    pub fn external(&self) -> usize {
        self.external
    }

    /// Its internal link rank from 0 to 100: 100 for the page the links
    /// favour most, and the others in proportion.
    pub fn rank(&self) -> u8 {
        self.rank
    }
}

/// The internal linking report of one crawl, every HTML page in crawl
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkReport {
    pages: Vec<PageLinks>,
}

impl LinkReport {
    /// The report for `crawl`.
    pub fn of(crawl: &Crawl) -> Self {
        let pages: Vec<&AbsoluteUrl> = crawl
            .visits()
            .iter()
            .filter(|visit| visit.page().is_some())
            .map(crate::Visit::url)
            .collect();
        let at: BTreeMap<&str, usize> = pages
            .iter()
            .enumerate()
            .map(|(index, url)| (url.as_str(), index))
            .collect();
        let mut to: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); pages.len()];
        let mut external = vec![0; pages.len()];
        for (from, url) in pages.iter().enumerate() {
            let page = crawl.visit(url.as_str()).and_then(crate::Visit::page);
            for link in page.map(crate::Page::links).unwrap_or_default() {
                match at.get(link.to().as_str()) {
                    Some(&target) if target != from && !link.nofollow() => {
                        if let Some(set) = to.get_mut(from) {
                            set.insert(target);
                        }
                    }
                    None if !crawl.is_inside(link.to()) => add_one(&mut external, from),
                    _ => {}
                }
            }
        }
        let ranks = ranks(&to);
        let mut inbound = vec![0; pages.len()];
        for target in to.iter().flatten() {
            add_one(&mut inbound, *target);
        }
        let pages = pages
            .iter()
            .enumerate()
            .map(|(index, url)| PageLinks {
                url: (*url).clone(),
                depth: crawl.visit(url.as_str()).and_then(crate::Visit::depth),
                inbound: inbound.get(index).copied().unwrap_or(0),
                outbound: to.get(index).map_or(0, BTreeSet::len),
                external: external.get(index).copied().unwrap_or(0),
                rank: ranks.get(index).copied().unwrap_or(0),
            })
            .collect();
        Self { pages }
    }

    /// Every page, in crawl order.
    pub fn pages(&self) -> &[PageLinks] {
        &self.pages
    }

    /// How many pages sit at each click depth, the link distribution.
    /// Pages only a sitemap leads to are under `None`.
    pub fn distribution(&self) -> BTreeMap<Option<u32>, usize> {
        let mut counts = BTreeMap::new();
        for page in &self.pages {
            let count: &mut usize = counts.entry(page.depth).or_default();
            *count = count.saturating_add(1);
        }
        counts
    }

    /// The report as CSV with a header row, one page per line.
    pub fn csv(&self) -> String {
        let mut csv = String::from("url,depth,inbound,outbound,external,rank\n");
        for page in &self.pages {
            let depth = page
                .depth
                .map(|clicks| clicks.to_string())
                .unwrap_or_default();
            let url = page.url.as_str();
            let quoted = if url.contains([',', '"']) {
                format!("\"{}\"", url.replace('"', "\"\""))
            } else {
                url.to_owned()
            };
            let _ = writeln!(
                csv,
                "{quoted},{depth},{},{},{},{}",
                page.inbound, page.outbound, page.external, page.rank
            );
        }
        csv
    }
}

/// Adds one to the count at `index`.
fn add_one(counts: &mut [usize], index: usize) {
    if let Some(count) = counts.get_mut(index) {
        *count = count.saturating_add(1);
    }
}

/// Each page's rank from 0 to 100, spread over the links `to` in whole
/// numbers: each round, every page keeps a little of the total and passes
/// the rest on in equal shares through its links.
fn ranks(to: &[BTreeSet<usize>]) -> Vec<u8> {
    let count = u64::try_from(to.len()).unwrap_or(u64::MAX);
    let Some(start) = TOTAL.checked_div(count) else {
        return Vec::new();
    };
    let kept = start.saturating_mul(100 - PASSED_PERCENT) / 100;
    let mut rank = vec![start; to.len()];
    for _ in 0..ROUNDS {
        let mut next = vec![kept; to.len()];
        for (from, targets) in to.iter().enumerate() {
            let share = u64::try_from(targets.len())
                .ok()
                .and_then(|links| rank.get(from)?.checked_div(links))
                .unwrap_or(0)
                .saturating_mul(PASSED_PERCENT)
                / 100;
            for target in targets {
                if let Some(value) = next.get_mut(*target) {
                    *value = value.saturating_add(share);
                }
            }
        }
        rank = next;
    }
    let most = rank.iter().copied().max().unwrap_or(0);
    rank.iter()
        .map(|value| {
            let scaled = value.saturating_mul(100).checked_div(most).unwrap_or(0);
            u8::try_from(scaled).unwrap_or(100)
        })
        .collect()
}
