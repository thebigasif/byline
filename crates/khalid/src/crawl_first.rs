//! `khalid crawl first`: how long each new post waited for each checked
//! crawler (GEO C2). The posts come from the Content API, the first
//! fetches from the crawled pages the visitor log keeps.

use std::fmt::Write as _;
use std::path::Path;

use chrono::{DateTime, Duration};
use khalid_analytics::{FirstCrawl, LivePage};
use khalid_api::content_store::ContentStore;
use khalid_api::query::{ListQuery, RawListQuery};
use khalid_api::resources::Collection;
use khalid_core::{Config, UserMessage};
use percent_encoding::percent_decode_str;
use serde::Serialize;
use serde_json::Value;

use crate::crawl_report::{shown, time};
use crate::crawl_words::CHECKED;
use crate::db_message;
use crate::mcp_content::FreshContent;

/// One post as `--json` prints it.
#[derive(Serialize)]
pub(crate) struct PostBody<'a> {
    path: &'a str,
    published_at: String,
    first_crawls: Vec<FetchBody>,
}

/// One crawler's first fetch as `--json` prints it.
#[derive(Serialize)]
struct FetchBody {
    crawler: &'static str,
    at: String,
    after_seconds: i64,
}

/// Every published post of the site in `folder`, as the Content API lists
/// it, with its address and when it went live.
pub(crate) async fn live_pages(
    folder: &Path,
    config: &Config,
) -> Result<Vec<LivePage>, UserMessage> {
    let mut raw = RawListQuery::default();
    raw.limit = Some("all".to_owned());
    let query = ListQuery::try_from(&raw).map_err(|_| db_message::internal())?;
    let content = FreshContent::new(folder, config.clone());
    let listed = content
        .list(Collection::Posts, &query)
        .await
        .map_err(|_| posts_unread())?;
    let items = listed.into_items();
    let pages = items.iter().filter_map(|post| {
        let shown = serde_json::to_value(post).ok()?;
        let text = |field: &str| shown.get(field).and_then(Value::as_str).map(str::to_owned);
        let published = DateTime::parse_from_rfc3339(&text("published_at")?).ok()?;
        Some(LivePage {
            path: path_of(&text("url")?)?,
            published_at: published.to_utc(),
        })
    });
    Ok(pages.collect())
}

/// The address part of a post's `url`, decoded, as the visitor log keeps
/// it, such as `/blog/geo/`.
fn path_of(url: &str) -> Option<String> {
    let (_, rest) = url.split_once("://")?;
    let (_, path) = rest.split_once('/')?;
    let path = path.split(['?', '#']).next()?;
    let path = percent_decode_str(path).decode_utf8().ok()?;
    Some(format!("/{path}"))
}

/// `posts` as `--json` prints them.
pub(crate) fn bodies(posts: &[FirstCrawl]) -> Vec<PostBody<'_>> {
    posts
        .iter()
        .map(|post| PostBody {
            path: &post.path,
            published_at: time(post.published_at),
            first_crawls: post
                .crawlers
                .iter()
                .map(|fetch| FetchBody {
                    crawler: fetch.crawler.name(),
                    at: time(fetch.at),
                    after_seconds: fetch.after.num_seconds(),
                })
                .collect(),
        })
        .collect()
}

/// Each post's first crawls as sentences, newest post first.
pub(crate) fn words(posts: &[FirstCrawl]) -> String {
    if posts.is_empty() {
        return format!(
            "No post has gone live since the visitor log saw its first crawl.\n\
             Posts that went live before that are left out, since their first crawl may have come earlier.\n\n{CHECKED}"
        );
    }
    let mut lines =
        String::from("When checked crawlers first fetched each post, newest post first.\n\n");
    for post in posts {
        let _ = writeln!(lines, "{}  live {}", post.path, shown(post.published_at));
        if post.crawlers.is_empty() {
            lines.push_str("  No checked crawler has fetched it yet.\n");
            continue;
        }
        let fetches: Vec<String> = post
            .crawlers
            .iter()
            .map(|fetch| format!("{} after {}", fetch.crawler.name(), span(fetch.after)))
            .collect();
        let _ = writeln!(lines, "  {}", fetches.join(", "));
    }
    lines.push('\n');
    lines.push_str(CHECKED);
    lines
}

/// `after` in the largest whole unit that reads well: minutes under an
/// hour, hours under two days, else days.
fn span(after: Duration) -> String {
    let (count, unit) = match after {
        short if short < Duration::hours(1) => (short.num_minutes(), "minute"),
        middle if middle < Duration::hours(48) => (middle.num_hours(), "hour"),
        long => (long.num_days(), "day"),
    };
    if count == 1 {
        format!("1 {unit}")
    } else {
        format!("{count} {unit}s")
    }
}

/// The answer when the posts cannot be read for their first crawls.
fn posts_unread() -> UserMessage {
    UserMessage::new(
        "Khalid could not read the site's posts.",
        "Without them, it cannot tell when each post went live.",
        "Run khalid lint to see what is wrong with the content, fix it, then run this again.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_post_address_is_its_decoded_path_without_query() {
        let url = "https://www.example.com/blog/two%20words/?ref=feed";
        assert_eq!(path_of(url).as_deref(), Some("/blog/two words/"));
        assert_eq!(path_of("https://www.example.com").as_deref(), None);
    }

    #[test]
    fn a_wait_reads_in_minutes_hours_or_days() {
        assert_eq!(span(Duration::minutes(1)), "1 minute");
        assert_eq!(span(Duration::minutes(59)), "59 minutes");
        assert_eq!(span(Duration::hours(47)), "47 hours");
        assert_eq!(span(Duration::hours(48)), "2 days");
    }
}
