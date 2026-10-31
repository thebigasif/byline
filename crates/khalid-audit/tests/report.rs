//! The health score, the counts, the CSV export and the trend.

pub mod support;

use chrono::{TimeZone, Utc};
use khalid_audit::{Category, Check, Report, Run, Severity, Thresholds, history};
use support::{HOME, crawled, page};

/// A page with nothing for the checks to find.
#[cfg(test)]
fn clean(links: &str) -> khalid_audit::Answer {
    let head = r#"<title>Why internal links matter</title>
<meta name="description" content="Internal links help readers find the rest of your site.">
<meta property="og:title" content="Links"><meta property="og:type" content="article">
<meta property="og:image" content="https://www.example.com/blog/share.png">
<meta property="og:url" content="https://www.example.com/blog/">
<meta name="twitter:card" content="summary">
<script type="application/ld+json">{"@type":"BlogPosting"}</script>"#;
    let body = format!("<h1>Links</h1><p>{}</p>{links}", "word ".repeat(300));
    page(head, &body)
}

#[tokio::test]
async fn a_blog_with_no_errors_scores_100() {
    let crawl = crawled(&[(HOME, clean(""))]).await;
    let report = Report::of(&crawl, Thresholds::default());
    assert_eq!(report.score(), 100);
    assert_eq!(report.pages(), 1);
    assert_eq!(report.count(Severity::Error), 0);
}

#[tokio::test]
async fn the_score_is_the_share_of_pages_without_an_error() {
    // Home links to three pages: one fine, one with no title, one missing.
    let crawl = crawled(&[
        (
            HOME,
            clean(r#"<a href="fine/">Fine</a><a href="untitled/">Untitled</a><a href="gone/">Gone</a>"#),
        ),
        ("https://www.example.com/blog/fine/", clean("")),
        ("https://www.example.com/blog/untitled/", page("", "<h1>No title</h1>")),
    ])
    .await;
    let report = Report::of(&crawl, Thresholds::default());
    // Home links to a broken page, the untitled page has no title, the
    // gone page is a 404: only the fine page is healthy, one of four.
    assert_eq!(report.pages(), 4);
    assert_eq!(report.score(), 25);
    let titles = report.by_category();
    assert_eq!(
        titles
            .get(&Category::Crawlability)
            .and_then(|checks| checks.get(&Check::Page4xx)),
        Some(&1)
    );
}

#[tokio::test]
async fn a_crawl_with_no_page_scores_0() {
    let crawl = crawled(&[]).await;
    assert_eq!(Report::of(&crawl, Thresholds::default()).score(), 0);
}

#[tokio::test]
async fn the_csv_has_a_header_and_quotes_what_a_spreadsheet_needs() {
    let crawl = crawled(&[(HOME, clean(r#"<img src="a, b.png">"#))]).await;
    let csv = Report::of(&crawl, Thresholds::default()).csv();
    let mut lines = csv.lines();
    assert_eq!(
        lines.next(),
        Some("severity,category,check,url,detail,why,fix")
    );
    let row = lines
        .find(|line| line.contains("Missing alt text"))
        .unwrap();
    assert_eq!(
        row,
        "warning,Images,Missing alt text,https://www.example.com/blog/,\"https://www.example.com/blog/a,%20b.png\",\"Screen readers and search engines cannot see an image, so they read its alt text instead.\",\"Describe each image in a few words, or give it an empty alt when it is decoration.\""
    );
}

#[tokio::test]
async fn each_run_is_one_line_and_the_history_skips_a_broken_line() {
    let crawl = crawled(&[(HOME, clean(""))]).await;
    let report = Report::of(&crawl, Thresholds::default());
    let at = Utc.with_ymd_and_hms(2026, 10, 27, 3, 0, 0).unwrap();
    let line = Run::of(&report, at).line().unwrap();
    // The test site has no sitemap, robots.txt or llms.txt: one warning
    // and two notices.
    assert_eq!(
        line,
        "{\"at\":\"2026-10-27T03:00:00Z\",\"score\":100,\"pages\":1,\"errors\":0,\"warnings\":1,\"notices\":2}\n"
    );
    let file = format!("{line}{{\"at\":\"2026-10-2\n{line}");
    let runs = history(&file);
    assert_eq!(runs.len(), 2);
    assert_eq!(runs.first().map(|run| run.score), Some(100));
}
