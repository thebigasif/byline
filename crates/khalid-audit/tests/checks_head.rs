//! The checks a page answers from its head.

pub mod support;

use khalid_audit::{Answer, Category, Check, Crawl, Issue, Severity, Thresholds};
use support::{HOME, crawled, page};

const GOOD_HEAD: &str = r#"<title>Why internal links matter</title>
<meta name="description" content="Internal links help readers find the rest of your site.">"#;

/// The issues of the head checks in `crawl`, leaving out what the body
/// checks find on these short test pages.
#[cfg(test)]
fn audit(crawl: &Crawl) -> Vec<Issue> {
    let head = [
        Category::Indexability,
        Category::Titles,
        Category::Headings,
        Category::Localization,
    ];
    khalid_audit::audit(crawl, Thresholds::default())
        .into_iter()
        .filter(|issue| head.contains(&issue.check().category()))
        .collect()
}

#[cfg(test)]
async fn issues(head: &str, body: &str) -> Vec<(Check, String)> {
    let crawl = crawled(&[(HOME, page(head, body))]).await;
    audit(&crawl)
        .iter()
        .map(|issue| (issue.check(), issue.detail().to_owned()))
        .collect()
}

#[tokio::test]
async fn a_good_head_has_no_issues() {
    assert_eq!(issues(GOOD_HEAD, "<h1>Why links matter</h1>").await, []);
}

#[tokio::test]
async fn noindex_from_a_tag_or_a_header_is_named() {
    let head = format!(r#"{GOOD_HEAD}<meta name="robots" content="noindex">"#);
    assert_eq!(
        issues(&head, "<h1>Links</h1>").await,
        [(Check::Noindex, String::new())]
    );
    let head = format!(r#"{GOOD_HEAD}<meta name="robots" content="none">"#);
    assert_eq!(
        issues(&head, "<h1>Links</h1>").await,
        [(Check::NoindexNofollow, String::new())]
    );
    let answer = page(GOOD_HEAD, "<h1>Links</h1>").with_header("x-robots-tag", "noindex");
    let crawl = crawled(&[(HOME, answer)]).await;
    let found = audit(&crawl);
    let issue = found.first().unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(issue.check(), Check::Noindex);
    assert_eq!(issue.url().as_str(), HOME);
}

#[tokio::test]
async fn a_canonical_elsewhere_is_named_and_one_to_itself_is_not() {
    let own = format!(r#"{GOOD_HEAD}<link rel="canonical" href="{HOME}">"#);
    assert_eq!(issues(&own, "<h1>Links</h1>").await, []);
    let other = "https://www.example.com/blog/other/";
    let pointing = format!(r#"{GOOD_HEAD}<link rel="canonical" href="{other}">"#);
    let crawl = crawled(&[
        (HOME, page(&pointing, "<h1>Links</h1>")),
        (other, page("<title>Other</title>", "<h1>Other</h1>")),
    ])
    .await;
    let issues = audit(&crawl);
    let found: Vec<(Check, &str, &str)> = issues
        .iter()
        .filter(|issue| issue.url().as_str() == HOME)
        .map(|issue| (issue.check(), issue.url().as_str(), issue.detail()))
        .collect();
    assert_eq!(found, [(Check::NonCanonical, HOME, other)]);
}

#[tokio::test]
async fn a_page_kept_out_of_search_needs_no_description() {
    let head = r#"<title>Search</title><meta name="robots" content="noindex">"#;
    assert_eq!(
        issues(head, "<h1>Search</h1>").await,
        [(Check::Noindex, String::new())]
    );
}

#[tokio::test]
async fn a_missing_title_description_and_heading_are_each_named() {
    assert_eq!(
        issues("<title> </title>", "<h1></h1>").await,
        [
            (Check::TitleMissing, String::new()),
            (Check::DescriptionMissing, String::new()),
            (Check::HeadingMissing, String::new()),
        ]
    );
}

#[tokio::test]
async fn two_of_each_are_named_with_the_count() {
    let head = format!("{GOOD_HEAD}{GOOD_HEAD}");
    assert_eq!(
        issues(&head, "<h1>One</h1><h1>Two</h1>").await,
        [
            (Check::TitleMultiple, "2".to_owned()),
            (Check::DescriptionMultiple, "2".to_owned()),
            (Check::HeadingMultiple, "2".to_owned()),
        ]
    );
}

#[tokio::test]
async fn a_long_title_and_description_say_their_length() {
    let title = "word ".repeat(15);
    let description = "word ".repeat(40);
    let head =
        format!(r#"<title>{title}</title><meta name="description" content="{description}">"#);
    assert_eq!(
        issues(&head, "<h1>Links</h1>").await,
        [
            (Check::TitleTooLong, "74 characters".to_owned()),
            (Check::DescriptionTooLong, "199 characters".to_owned()),
        ]
    );
}

#[tokio::test]
async fn a_page_without_a_language_is_named() {
    let html =
        format!("<!doctype html><html><head>{GOOD_HEAD}</head><body><h1>Links</h1></body></html>");
    let answer = Answer::new(200, html.into_bytes()).with_header("content-type", "text/html");
    let crawl = crawled(&[(HOME, answer)]).await;
    let checks: Vec<Check> = audit(&crawl).iter().map(Issue::check).collect();
    assert_eq!(checks, [Check::LanguageMissing]);
}

#[test]
fn every_check_has_an_identity_a_group_a_name_a_reason_and_a_fix() {
    let mut ids: Vec<&str> = Check::ALL.iter().map(|check| check.id()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), Check::ALL.len(), "two checks share an identity");
    for check in Check::ALL {
        assert_ne!(check.title(), "", "{check:?} has no name");
        assert_ne!(check.category().title(), "");
        for said in [check.why(), check.fix()] {
            assert!(said.ends_with('.'), "{check:?}: {said}");
            assert!(!said.contains('\u{2014}'), "{check:?} has an em dash");
        }
        assert_ne!(check.why(), check.fix(), "{check:?}");
    }
    assert_eq!(Check::NoindexNofollow.severity(), Severity::Warning);
    assert_eq!(Severity::Notice.as_str(), "notice");
}
