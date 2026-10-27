//! The crawl itself: breadth first from the home page, so each address is
//! found at its fewest clicks.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use khalid_core::AbsoluteUrl;
use url::Url;

use crate::{Outcome, Source, Visit};

/// How far a crawl may go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limit {
    pages: usize,
}

impl Default for Limit {
    /// 50,000 addresses, five times the largest site the budgets measure.
    fn default() -> Self {
        Self { pages: 50_000 }
    }
}

impl Limit {
    /// At most `pages` addresses inside the blog.
    pub fn pages(pages: usize) -> Self {
        Self { pages }
    }
}

/// Everything one crawl found.
#[derive(Debug, Clone)]
pub struct Crawl {
    home: AbsoluteUrl,
    visits: Vec<Visit>,
    index: BTreeMap<String, usize>,
    outside: BTreeSet<AbsoluteUrl>,
    stopped_at_limit: bool,
}

impl Crawl {
    /// The home page the crawl started from.
    pub fn home(&self) -> &AbsoluteUrl {
        &self.home
    }

    /// Every address inside the blog, in the order the crawl asked for it.
    pub fn visits(&self) -> &[Visit] {
        &self.visits
    }

    /// The visit to the address `url`, when the crawl asked for it.
    pub fn visit(&self, url: &str) -> Option<&Visit> {
        self.visits.get(*self.index.get(url)?)
    }

    /// Every address outside the blog a page links to or loads, sorted.
    /// They are checked, never crawled.
    pub fn outside(&self) -> &BTreeSet<AbsoluteUrl> {
        &self.outside
    }

    /// Whether the crawl stopped at its [`Limit`] with addresses left.
    pub fn stopped_at_limit(&self) -> bool {
        self.stopped_at_limit
    }

    /// Whether `url` is inside the blog: the home page's scheme, host and
    /// port, and a path under the home page's.
    pub fn is_inside(&self, url: &AbsoluteUrl) -> bool {
        inside(&self.home, url)
    }
}

/// Crawls the blog whose home page is `home`, asking `source` for every
/// address inside it that a page links to or loads, until `limit`.
pub async fn crawl<S: Source>(source: &S, home: &AbsoluteUrl, limit: Limit) -> Crawl {
    let mut found = Crawl {
        home: home.clone(),
        visits: Vec::new(),
        index: BTreeMap::new(),
        outside: BTreeSet::new(),
        stopped_at_limit: false,
    };
    let mut queue = VecDeque::from([(home.clone(), 0)]);
    let mut seen = BTreeSet::from([home.as_str().to_owned()]);
    while let Some((url, depth)) = queue.pop_front() {
        if found.visits.len() >= limit.pages {
            found.stopped_at_limit = true;
            break;
        }
        let visit = match source.fetch(&url).await {
            Ok(answer) => Visit::answered(url, depth, &answer),
            Err(why) => Visit::unreached(url, depth, why),
        };
        let mut next = Vec::new();
        if let Outcome::Redirect { to, .. } = visit.outcome() {
            next.push((to.clone(), depth));
        }
        if let Some(page) = visit.page() {
            let links = page.links().iter().map(crate::Link::to);
            for to in links.chain(page.resources()) {
                next.push((to.clone(), depth.saturating_add(1)));
            }
        }
        for (to, at) in next {
            if !found.is_inside(&to) {
                found.outside.insert(to);
            } else if seen.insert(to.as_str().to_owned()) {
                queue.push_back((to, at));
            }
        }
        found
            .index
            .insert(visit.url().as_str().to_owned(), found.visits.len());
        found.visits.push(visit);
    }
    found
}

/// Whether `url` shares `home`'s scheme, host and port and sits under its
/// path.
fn inside(home: &AbsoluteUrl, url: &AbsoluteUrl) -> bool {
    match (Url::parse(home.as_str()), Url::parse(url.as_str())) {
        (Ok(home), Ok(url)) => home.origin() == url.origin() && url.path().starts_with(home.path()),
        _ => false,
    }
}
