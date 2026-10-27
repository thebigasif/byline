//! The checks a page answers from its head.

pub mod support;

use khalid_audit::{Check, Severity, audit};
use support::{HOME, crawled, page};

const GOOD_HEAD: &str = r#"<title>Why internal links matter</title>
<meta name="description" content="Internal links help readers find the rest of your site.">"#;

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
    let other =
        format!(r#"{GOOD_HEAD}<link rel="canonical" href="https://www.example.com/blog/other/">"#);
    assert_eq!(
        issues(&other, "<h1>Links</h1>").await,
        [(
            Check::NonCanonical,
            "https://www.example.com/blog/other/".to_owned()
        )]
    );
}

#[test]
fn every_check_has_an_identity_a_group_a_name_and_a_fix() {
    let mut ids: Vec<&str> = Check::ALL.iter().map(|check| check.id()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), Check::ALL.len(), "two checks share an identity");
    for check in Check::ALL {
        assert_ne!(check.title(), "", "{check:?} has no name");
        assert_ne!(check.category().title(), "");
        let fix = check.fix();
        assert!(fix.ends_with('.'), "{check:?}: {fix}");
        assert!(!fix.contains('\u{2014}'), "{check:?} has an em dash");
    }
    assert_eq!(Check::NoindexNofollow.severity(), Severity::Warning);
    assert_eq!(Severity::Notice.as_str(), "notice");
}
