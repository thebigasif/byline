//! `khalid search-console post <slug>` and `khalid search-console index`:
//! one post's numbers with the searches behind them and its index status,
//! and the index status of every post (plan 5.11).

use std::fmt::Write as _;
use std::path::Path;

use chrono::NaiveDate;
use khalid_content::SiteContent;
use khalid_core::{AbsoluteUrl, Config, Slug, UserMessage};
use khalid_outbound::Outbound;
use khalid_search_console::{
    IndexStatus, PageNumbers, Question, SearchConsole, Verdict, go_deeper,
};
use serde::Serialize;

use crate::bing::Failed;
use crate::cli::Output;
use crate::exit::Exit;
use crate::report;
use crate::search_console::{ask, last_month};
use crate::search_console_report::{LivePost, live_posts};

/// How many searches a post's report lists.
const QUERIES: u32 = 10;

/// The most posts one `index` run asks about, a tenth of Google's daily
/// quota of 2,000 for a property.
const MOST_INSPECTED: usize = 200;

/// The JSON shape of one post's report.
#[derive(Serialize)]
struct PostBody<'a> {
    title: &'a str,
    url: &'a str,
    first: String,
    last: String,
    /// What Google counted for the post, by search.
    queries: Vec<QueryBody>,
    index: IndexBody,
    /// Where to ask what to write next.
    go_deeper: String,
}

/// The JSON shape of one search's numbers.
#[derive(Serialize)]
struct QueryBody {
    query: String,
    clicks: f64,
    impressions: f64,
    ctr: f64,
    position: f64,
}

/// The JSON shape of a post's index status.
#[derive(Serialize)]
struct IndexBody {
    title: String,
    url: String,
    indexed: bool,
    /// `pass`, `partial`, `neutral`, `fail` or `unknown`, Google's verdict.
    verdict: &'static str,
    /// Google's own words, such as `Submitted and indexed`.
    coverage: String,
    last_crawl: Option<String>,
}

/// Reads the post `slug` of the checked site from `console` up to `today`
/// and prints it.
pub(crate) fn post(
    (folder, config, content): (&Path, &Config, &SiteContent),
    slug: &Slug,
    (console, outbound, today): (&SearchConsole, &Outbound, NaiveDate),
    output: Output,
) -> Result<Exit, Failed> {
    let posts = live_posts(folder, config, content)?;
    let post = posts
        .iter()
        .find(|post| &post.slug == slug)
        .ok_or_else(|| (no_post(slug), Exit::Usage))?;
    let url = address(&post.address)?;
    let month = last_month(today);
    let (queries, status) = ask(console, async {
        let session = console.sign_in(outbound).await?;
        tokio::try_join!(
            session.queries(month, &post.address, QUERIES),
            session.inspect(&url)
        )
    })?;
    let body = PostBody {
        title: &post.title,
        url: &post.address,
        first: month.first.to_string(),
        last: month.last.to_string(),
        queries: queries.iter().map(query_body).collect(),
        index: index_body(post, &status),
        go_deeper: go_deeper(Question::WhatToWriteNext, &post.address),
    };
    Ok(match output {
        Output::Json => report::json(&body, Exit::Success),
        Output::Human => report::text(&post_sentences(&body), Exit::Success),
    })
}

/// Asks `console` whether each post of the checked site is in Google's
/// index, the first 200 in the content index's order, and prints it.
pub(crate) fn index(
    (folder, config, content): (&Path, &Config, &SiteContent),
    (console, outbound): (&SearchConsole, &Outbound),
    output: Output,
) -> Result<Exit, Failed> {
    let posts = live_posts(folder, config, content)?;
    let asked: Vec<&LivePost> = posts.iter().take(MOST_INSPECTED).collect();
    let urls = asked
        .iter()
        .map(|post| address(&post.address))
        .collect::<Result<Vec<_>, _>>()?;
    let statuses = ask(console, async {
        let session = console.sign_in(outbound).await?;
        let mut found = Vec::with_capacity(urls.len());
        for url in &urls {
            found.push(session.inspect(url).await?);
        }
        Ok(found)
    })?;
    let bodies: Vec<IndexBody> = asked
        .iter()
        .zip(&statuses)
        .map(|(post, status)| index_body(post, status))
        .collect();
    Ok(match output {
        Output::Json => report::json(&bodies, Exit::Success),
        Output::Human => report::text(&index_sentences(&bodies), Exit::Success),
    })
}

/// `text`, a post's address made from the site's own, as one.
fn address(text: &str) -> Result<AbsoluteUrl, Failed> {
    AbsoluteUrl::parse(text).map_err(|_| (crate::serve_message::internal(), Exit::Internal))
}

fn query_body(row: &PageNumbers) -> QueryBody {
    QueryBody {
        query: row.key.clone(),
        clicks: row.clicks,
        impressions: row.impressions,
        ctr: row.ctr,
        position: row.position,
    }
}

fn index_body(post: &LivePost, status: &IndexStatus) -> IndexBody {
    IndexBody {
        title: post.title.clone(),
        url: post.address.clone(),
        indexed: status.is_indexed(),
        verdict: match status.verdict {
            Verdict::Pass => "pass",
            Verdict::Partial => "partial",
            Verdict::Neutral => "neutral",
            Verdict::Fail => "fail",
            _ => "unknown",
        },
        coverage: status.coverage_state.clone(),
        last_crawl: status.last_crawl_time.clone(),
    }
}

/// A post's index status in one sentence.
fn said(index: &IndexBody) -> String {
    let crawled = index
        .last_crawl
        .as_deref()
        .and_then(|time| time.get(..10))
        .map_or_else(
            || "never crawled".to_owned(),
            |day| format!("last crawled {day}"),
        );
    let state = if index.indexed { "in" } else { "not in" };
    format!("{state} Google's index ({}), {crawled}", index.coverage)
}

/// One post's report as sentences for a person.
fn post_sentences(body: &PostBody<'_>) -> String {
    let mut text = format!(
        "{}\n{}\nIt is {}.\n",
        body.title,
        body.url,
        said(&body.index)
    );
    if body.queries.is_empty() {
        let _ = writeln!(
            text,
            "No search showed it from {} to {}.",
            body.first, body.last
        );
    } else {
        let _ = writeln!(
            text,
            "Searches that showed it, {} to {}:",
            body.first, body.last
        );
    }
    for query in &body.queries {
        // Writing to a String cannot fail, so the results are dropped.
        let _ = writeln!(
            text,
            "- {}: {:.0} clicks, {:.0} impressions, average position {:.1}.",
            query.query, query.clicks, query.impressions, query.position
        );
    }
    let _ = writeln!(
        text,
        "What to write next? Go deeper in SerpDad: {}",
        body.go_deeper
    );
    text
}

/// Every post's index status as sentences for a person.
fn index_sentences(bodies: &[IndexBody]) -> String {
    let indexed = bodies.iter().filter(|body| body.indexed).count();
    let mut text = format!(
        "{indexed} of {} posts are in Google's index.\n",
        bodies.len()
    );
    for body in bodies {
        let _ = writeln!(text, "- {}: {}.", body.title, said(body));
    }
    text
}

/// The message when no published post has `slug`.
fn no_post(slug: &Slug) -> UserMessage {
    UserMessage::new(
        format!("No published post has the slug {slug}."),
        "Search Console counts only pages readers can open, so a draft has no numbers.",
        "Check the slug in the post's front matter, then run the command again.",
    )
}
