//! When each new post was first fetched by each checked crawler (GEO C2),
//! from the crawled pages the visitor log keeps.

use std::cmp::Reverse;

use chrono::{DateTime, Duration, Utc};

use crate::{CrawledPage, Crawler};

/// A published post: its address and when it went live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivePage {
    /// The address, such as `/blog/geo/`.
    pub path: String,
    /// When it went live.
    pub published_at: DateTime<Utc>,
}

/// When each checked crawler first fetched one post.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstCrawl {
    /// The post's address.
    pub path: String,
    /// When it went live.
    pub published_at: DateTime<Utc>,
    /// Each crawler's first fetch, earliest first. Empty when none came.
    pub crawlers: Vec<FirstFetch>,
}

/// One crawler's first fetch of a post.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstFetch {
    /// The crawler, checked against its company's list.
    pub crawler: Crawler,
    /// When it first fetched the post.
    pub at: DateTime<Utc>,
    /// How long after the post went live, zero when the address was
    /// fetched before.
    pub after: Duration,
}

/// The first fetches of each post in `live` that went live after the
/// first page in `crawled` was fetched, newest post first. A post that went
/// live before the visitor log saw any crawl is left out, since its first
/// crawl may have come before the log was on.
pub fn first_crawls(live: &[LivePage], crawled: &[CrawledPage]) -> Vec<FirstCrawl> {
    let Some(start) = crawled.iter().map(|page| page.first_at).min() else {
        return Vec::new();
    };
    let mut posts: Vec<FirstCrawl> = live
        .iter()
        .filter(|post| post.published_at >= start)
        .map(|post| {
            let mut crawlers: Vec<FirstFetch> = crawled
                .iter()
                .filter(|page| page.path == post.path)
                .map(|page| FirstFetch {
                    crawler: page.crawler,
                    at: page.first_at,
                    after: (page.first_at - post.published_at).max(Duration::zero()),
                })
                .collect();
            crawlers.sort_by_key(|fetch| (fetch.at, fetch.crawler));
            FirstCrawl {
                path: post.path.clone(),
                published_at: post.published_at,
                crawlers,
            }
        })
        .collect();
    posts.sort_by(|a, b| {
        (Reverse(a.published_at), &a.path).cmp(&(Reverse(b.published_at), &b.path))
    });
    posts
}
