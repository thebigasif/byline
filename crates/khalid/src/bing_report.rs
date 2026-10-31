//! `khalid bing report`: Bing's last crawl day, the sitemaps Bing holds and
//! the addresses it had trouble with, as sentences or JSON.

use std::fmt::Write as _;

use khalid_core::Config;
use khalid_outbound::Outbound;
use khalid_seo::{BingSitemap, BingWebmaster, CrawlDay, CrawlIssue, CrawlIssueKind};
use serde::Serialize;

use crate::bing::{Failed, block_on, failed};
use crate::cli::Output;
use crate::exit::Exit;
use crate::report;

/// The JSON shape of a report.
#[derive(Serialize)]
struct ReportBody {
    /// The latest day Bing has counts for, if any.
    last_day: Option<DayBody>,
    sitemaps: Vec<SitemapBody>,
    issues: Vec<IssueBody>,
}

/// The JSON shape of one crawl day.
#[derive(Serialize)]
struct DayBody {
    date: String,
    crawled_pages: u64,
    in_index: u64,
    crawl_errors: u64,
    blocked_by_robots_txt: u64,
}

/// The JSON shape of one sitemap Bing holds.
#[derive(Serialize)]
struct SitemapBody {
    url: String,
    status: String,
    url_count: u64,
    last_read: Option<String>,
}

/// The JSON shape of one address with crawl issues.
#[derive(Serialize)]
struct IssueBody {
    url: String,
    http_code: u16,
    /// Each kind's name, such as `code_4xx`.
    kinds: Vec<&'static str>,
    /// What each kind means, in plain words, for the sentences.
    #[serde(skip)]
    said: Vec<&'static str>,
}

/// Reads the three lists from Bing about the site with `config` and
/// prints them.
pub(crate) fn run(
    config: &Config,
    bing: &BingWebmaster,
    outbound: &Outbound,
    output: Output,
) -> Result<Exit, Failed> {
    let read = block_on(async {
        tokio::try_join!(
            bing.crawl_stats(outbound),
            bing.sitemaps(outbound),
            bing.crawl_issues(outbound),
        )
    })?;
    let (days, sitemaps, issues) = read.map_err(|error| failed(config, &error))?;
    let body = ReportBody {
        last_day: days.iter().max_by_key(|day| day.date).map(day_body),
        sitemaps: sitemaps.iter().map(sitemap_body).collect(),
        issues: issues.iter().map(issue_body).collect(),
    };
    Ok(match output {
        Output::Json => report::json(&body, Exit::Success),
        Output::Human => report::text(&sentences(&body), Exit::Success),
    })
}

fn day_body(day: &CrawlDay) -> DayBody {
    DayBody {
        date: day.date.date_naive().to_string(),
        crawled_pages: day.crawled_pages,
        in_index: day.in_index,
        crawl_errors: day.crawl_errors,
        blocked_by_robots_txt: day.blocked_by_robots_txt,
    }
}

fn sitemap_body(sitemap: &BingSitemap) -> SitemapBody {
    SitemapBody {
        url: sitemap.url.clone(),
        status: sitemap.status.clone(),
        url_count: sitemap.url_count,
        last_read: sitemap.last_crawled.map(|at| at.date_naive().to_string()),
    }
}

fn issue_body(issue: &CrawlIssue) -> IssueBody {
    let (kinds, said) = issue.kinds().into_iter().map(words).unzip();
    IssueBody {
        url: issue.url.clone(),
        http_code: issue.http_code,
        kinds,
        said,
    }
}

/// The report as sentences for a person.
fn sentences(body: &ReportBody) -> String {
    let mut text = String::new();
    // Writing to a String cannot fail, so the results are dropped.
    let _ = match &body.last_day {
        Some(day) => writeln!(
            text,
            "Last crawl day, {}: {} pages fetched, {} in Bing's index, {} failed, {} kept out by robots.txt.",
            day.date, day.crawled_pages, day.in_index, day.crawl_errors, day.blocked_by_robots_txt
        ),
        None => writeln!(
            text,
            "Bing has no crawl days for this site yet. A site added in the last two days has none."
        ),
    };
    if body.sitemaps.is_empty() {
        text.push_str(
            "\nBing holds no sitemap for this site. Run khalid bing sitemap to hand it one.\n",
        );
    } else {
        text.push_str("\nSitemaps Bing holds:\n");
    }
    for sitemap in &body.sitemaps {
        let read = sitemap.last_read.as_deref().unwrap_or("not yet");
        let _ = writeln!(
            text,
            "- {}: {}, {} addresses, last read {read}.",
            sitemap.url, sitemap.status, sitemap.url_count
        );
    }
    if body.issues.is_empty() {
        text.push_str("\nBing reports no addresses it had trouble with.\n");
    } else {
        text.push_str("\nAddresses Bing had trouble with:\n");
    }
    for issue in &body.issues {
        let said = issue.said.join("; ");
        let _ = writeln!(text, "- {} ({}): {said}.", issue.url, issue.http_code);
    }
    text
}

/// A kind's JSON name and what it means, in plain words.
fn words(kind: CrawlIssueKind) -> (&'static str, &'static str) {
    match kind {
        CrawlIssueKind::Code301 => ("code_301", "moved for good with a 301 redirect"),
        CrawlIssueKind::Code302 => ("code_302", "moved for now with a 302 redirect"),
        CrawlIssueKind::Code4xx => (
            "code_4xx",
            "answered with a 4xx status, such as a missing page",
        ),
        CrawlIssueKind::Code5xx => ("code_5xx", "answered with a 5xx status, a server error"),
        CrawlIssueKind::BlockedByRobotsTxt => ("blocked_by_robots_txt", "kept out by robots.txt"),
        CrawlIssueKind::ContainsMalware => ("contains_malware", "Bing found malware on it"),
        CrawlIssueKind::ImportantUrlBlockedByRobotsTxt => (
            "important_url_blocked_by_robots_txt",
            "an address Bing counts as important, kept out by robots.txt",
        ),
        CrawlIssueKind::DnsErrors => ("dns_errors", "the host name could not be looked up"),
        CrawlIssueKind::TimeOutErrors => ("time_out_errors", "the server took too long to answer"),
        _ => ("other", "an issue Khalid does not know yet"),
    }
}
