//! The AI search checks on what a page shows: answers in view, its date
//! on the page, and whether the date is recent (phase GEO, A1, built on P1
//! and F1).

pub mod support;

use chrono::NaiveDate;
use khalid_audit::{Check, Thresholds, audit};
use support::{HOME, crawled, page};

/// The checks this file is about.
const CHECKS: [Check; 3] = [Check::HiddenAnswer, Check::DateNotShown, Check::StalePage];

/// The day these tests audit on.
#[cfg(test)]
fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()
}

/// An article's JSON-LD with `dates`, such as `"datePublished":"..."`.
#[cfg(test)]
fn article(dates: &str) -> String {
    format!(
        r#"<script type="application/ld+json">{{"@context":"https://schema.org","@graph":[{{"@type":"WebPage"}},{{"@type":"BlogPosting",{dates}}}]}}</script>"#
    )
}

/// This file's checks on one page with `head` and `body`.
#[cfg(test)]
async fn found(head: &str, body: &str, thresholds: Thresholds) -> Vec<(Check, String)> {
    let crawl = crawled(&[(HOME, page(head, body))]).await;
    audit(&crawl, thresholds)
        .iter()
        .filter(|issue| CHECKS.contains(&issue.check()))
        .map(|issue| (issue.check(), issue.detail().to_owned()))
        .collect()
}

#[tokio::test]
async fn an_answer_folded_away_is_named_by_its_question() {
    let body = "<details><summary>Can I freeze rye?</summary><p>Yes.</p></details>\
        <details open><summary>Shown</summary><p>Open.</p></details>";
    assert_eq!(
        found("", body, Thresholds::default()).await,
        [(Check::HiddenAnswer, "Can I freeze rye?".to_owned())]
    );
}

#[tokio::test]
async fn a_recent_article_with_its_date_shown_passes() {
    let head = article(
        r#""datePublished":"2026-03-01T00:00:00+00:00","dateModified":"2026-09-14T00:00:00+00:00""#,
    );
    let body = r#"<p>Published <time datetime="2026-03-01">then</time>. Updated <time datetime="2026-09-14">later</time>.</p>"#;
    let thresholds = Thresholds::default().with_today(today());
    assert_eq!(found(&head, body, thresholds).await, []);
}

#[tokio::test]
async fn the_latest_date_must_be_on_the_page() {
    let head = article(
        r#""datePublished":"2026-03-01T00:00:00+00:00","dateModified":"2026-09-14T00:00:00+00:00""#,
    );
    let body = r#"<p>Published <time datetime="2026-03-01">then</time>.</p>"#;
    assert_eq!(
        found(&head, body, Thresholds::default()).await,
        [(Check::DateNotShown, "2026-09-14".to_owned())]
    );
}

#[tokio::test]
async fn an_article_past_its_review_window_is_stale() {
    let head = article(r#""datePublished":"2025-10-30T08:00:00Z""#);
    let body = r#"<p><time datetime="2025-10-30">then</time></p>"#;
    let thresholds = Thresholds::default().with_today(today());
    assert_eq!(
        found(&head, body, thresholds).await,
        [(Check::StalePage, "2025-10-30".to_owned())]
    );
    let longer = thresholds.with_stale_months(13);
    assert_eq!(found(&head, body, longer).await, []);
    assert_eq!(
        found(&head, body, Thresholds::default()).await,
        [],
        "without a day to measure from, nothing is stale"
    );
}

#[tokio::test]
async fn a_page_that_is_no_article_has_no_date_to_show() {
    let head = r#"<script type="application/ld+json">{"@type":"WebSite","datePublished":"2020-01-01"}</script>"#;
    let thresholds = Thresholds::default().with_today(today());
    assert_eq!(found(head, "<p>Home</p>", thresholds).await, []);
}
