//! The checks a page answers from its body: text, links, images, social
//! tags and structured data.

pub mod support;

use khalid_audit::{Check, Thresholds, audit};
use support::{HOME, crawled, page};

/// A head with nothing for these checks to find.
const HEAD: &str = r#"<title>Why internal links matter</title>
<meta name="description" content="Internal links help readers find the rest of your site.">
<meta property="og:title" content="Why internal links matter">
<meta property="og:type" content="article">
<meta property="og:image" content="https://www.example.com/blog/share.png">
<meta property="og:url" content="https://www.example.com/blog/">
<meta name="twitter:card" content="summary">
<script type="application/ld+json">{"@type":"BlogPosting"}</script>"#;

#[cfg(test)]
fn words(count: usize) -> String {
    format!("<h1>Links</h1><p>{}</p>", "word ".repeat(count))
}

#[cfg(test)]
async fn found(head: &str, body: &str, thresholds: Thresholds) -> Vec<(Check, String)> {
    let crawl = crawled(&[(HOME, page(head, body))]).await;
    audit(&crawl, thresholds)
        .iter()
        .map(|issue| (issue.check(), issue.detail().to_owned()))
        .filter(|(check, _)| *check != Check::LowTextRatio)
        .collect()
}

#[tokio::test]
async fn a_full_page_has_no_issues() {
    let crawl = crawled(&[(HOME, page(HEAD, &words(400)))]).await;
    assert_eq!(audit(&crawl, Thresholds::default()), []);
}

#[tokio::test]
async fn a_thin_page_is_named_unless_it_is_kept_out_of_search() {
    let thin = found(HEAD, &words(20), Thresholds::default()).await;
    assert_eq!(thin, [(Check::LowWordCount, "21 words".to_owned())]);
    let hidden = format!(r#"{HEAD}<meta name="robots" content="noindex">"#);
    let checks: Vec<Check> = found(&hidden, &words(20), Thresholds::default())
        .await
        .into_iter()
        .map(|(check, _)| check)
        .collect();
    assert_eq!(checks, [Check::Noindex]);
}

#[tokio::test]
async fn markup_with_little_text_is_a_low_ratio() {
    let body = format!("{}<div>{}</div>", words(400), "<span></span>".repeat(2000));
    let crawl = crawled(&[(HOME, page(HEAD, &body))]).await;
    let checks: Vec<Check> = audit(&crawl, Thresholds::default())
        .iter()
        .map(khalid_audit::Issue::check)
        .collect();
    assert_eq!(checks, [Check::LowTextRatio]);
}

#[tokio::test]
async fn links_images_and_files_are_checked() {
    let body = format!(
        r#"{}<a href="/blog/next/" rel="nofollow">Next</a>
<a href="https://serpdad.com/" rel="nofollow">SerpDad</a>
<a href="http://old.example.org/">Old site</a>
<img src="/blog/a.png" alt=""><img src="/blog/b.png">
<script src="http://cdn.example.org/app.js"></script>"#,
        words(400)
    );
    let found = found(HEAD, &body, Thresholds::default().with_most_links(2)).await;
    assert_eq!(
        found,
        [
            (
                Check::NofollowInternal,
                "https://www.example.com/blog/next/".to_owned()
            ),
            (Check::TooManyLinks, "3 links".to_owned()),
            (
                Check::AltMissing,
                "https://www.example.com/blog/b.png".to_owned()
            ),
            (Check::HttpLink, "http://old.example.org/".to_owned()),
            (
                Check::MixedContent,
                "http://cdn.example.org/app.js".to_owned()
            ),
        ]
    );
}

#[tokio::test]
async fn missing_social_tags_and_structured_data_are_named() {
    let head = r#"<title>Links</title><meta name="description" content="About links.">
<meta property="og:title" content="Links">"#;
    let found = found(head, &words(400), Thresholds::default()).await;
    assert_eq!(
        found,
        [
            (
                Check::OpenGraphIncomplete,
                "og:type, og:image, og:url".to_owned()
            ),
            (Check::TwitterCardMissing, String::new()),
            (Check::StructuredDataMissing, String::new()),
        ]
    );
}

#[tokio::test]
async fn structured_data_that_is_not_json_is_named_with_its_place() {
    let head = format!(r#"{HEAD}<script type="application/ld+json">{{"@type": </script>"#);
    let found = found(&head, &words(400), Thresholds::default()).await;
    assert_eq!(
        found,
        [(Check::StructuredDataInvalid, "block 2".to_owned())]
    );
}
