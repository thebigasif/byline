//! Read but never cited: the pages AI bots read that no AI assistant sent
//! a reader to.
//!
//! A bot reading a page is not a citation. The only citation the server
//! can see is a reader arriving from an AI assistant, counted as a view in
//! [`ReferrerGroup::Ai`]. A page AI bots keep reading while no such reader
//! ever comes is one the assistants know but do not point people to.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};

use crate::{CrawlerKind, Crawls, ReferrerGroup, Views};

/// The status a bot got when it read a page that is not changed since its
/// last visit. It read the page before, so it counts as a read.
const NOT_MODIFIED: u16 = 304;

/// One page AI bots read in the period, with no reader from an AI
/// assistant in the same period.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Uncited {
    /// The page's path, such as `/blog/internal-links/`.
    pub path: String,
    /// How many times AI bots read it.
    pub reads: u64,
    /// The names of the AI bots that read it, in alphabetical order, as
    /// their user agents claimed.
    pub bots: Vec<String>,
    /// The latest read.
    pub last_read: DateTime<Utc>,
}

/// The pages in `crawls` that AI bots read, minus every page in `views`
/// that had a reader from an AI assistant, the most read first.
///
/// A read is a request from a [`CrawlerKind::Ai`] bot whose latest answer
/// that day was a success or "not modified". Requests for files, such as
/// `robots.txt` or an image, are not pages and are left out. Give both
/// lists for the same days.
pub fn uncited(crawls: &[Crawls], views: &[Views]) -> Vec<Uncited> {
    let cited: BTreeSet<&str> = views
        .iter()
        .filter(|row| matches!(row.group, ReferrerGroup::Ai(_)) && row.views > 0)
        .map(|row| row.path.as_str())
        .collect();
    let mut pages: BTreeMap<&str, Reads<'_>> = BTreeMap::new();
    let reads = crawls
        .iter()
        .filter(|row| row.kind == CrawlerKind::Ai && read_ok(row.last_status))
        .filter(|row| is_page(&row.path) && !cited.contains(row.path.as_str()));
    for row in reads {
        let page = pages.entry(&row.path).or_insert_with(|| Reads {
            hits: 0,
            bots: BTreeSet::new(),
            last: row.last_seen,
        });
        page.hits = page.hits.saturating_add(row.hits);
        page.bots.insert(&row.crawler);
        page.last = page.last.max(row.last_seen);
    }
    let mut listed: Vec<Uncited> = pages
        .into_iter()
        .map(|(path, page)| Uncited {
            path: path.to_owned(),
            reads: page.hits,
            bots: page.bots.into_iter().map(str::to_owned).collect(),
            last_read: page.last,
        })
        .collect();
    // Stable, so pages read as often stay in path order.
    listed.sort_by_key(|page| Reverse(page.reads));
    listed
}

/// The AI reads of one page, gathered across days and bots.
struct Reads<'a> {
    hits: u64,
    bots: BTreeSet<&'a str>,
    last: DateTime<Utc>,
}

/// Whether a bot answered with `status` got the page.
fn read_ok(status: u16) -> bool {
    (200..300).contains(&status) || status == NOT_MODIFIED
}

/// Whether `path` is a page rather than a file: its last part has no dot.
fn is_page(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_none_or(|last| !last.contains('.'))
}
