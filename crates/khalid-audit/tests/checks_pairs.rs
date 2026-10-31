//! The checks on what a page's head points at, canonical and `hreflang`,
//! and on titles and descriptions two pages share.

pub mod support;

use khalid_audit::{Answer, Check, Crawl, Thresholds, audit};
use support::{HOME, crawled, page};

const POST: &str = "https://www.example.com/blog/post/";
const GERMAN: &str = "https://www.example.com/blog/de/post/";

/// The issues of `checks` in `crawl`, as check, address and detail.
#[cfg(test)]
fn only(crawl: &Crawl, checks: &[Check]) -> Vec<(Check, String, String)> {
    audit(crawl, Thresholds::default())
        .iter()
        .filter(|issue| checks.contains(&issue.check()))
        .map(|issue| {
            let url = issue.url().as_str().to_owned();
            (issue.check(), url, issue.detail().to_owned())
        })
        .collect()
}

#[cfg(test)]
fn titled(title: &str, extra: &str) -> Answer {
    let head = format!(
        r#"<title>{title}</title><meta name="description" content="About {title}.">{extra}"#
    );
    page(&head, "<h1>A post</h1>")
}

#[cfg(test)]
fn alternate(language: &str, href: &str) -> String {
    format!(r#"<link rel="alternate" hreflang="{language}" href="{href}">"#)
}

#[tokio::test]
async fn the_crawl_follows_canonicals_and_translations() {
    let home = titled(
        "Home",
        &format!(
            r#"<link rel="canonical" href="{POST}">{}"#,
            alternate("de", GERMAN)
        ),
    );
    let crawl = crawled(&[(HOME, home)]).await;
    assert!(crawl.visit(POST).is_some());
    assert!(crawl.visit(GERMAN).is_some());
}

#[tokio::test]
async fn a_canonical_to_a_broken_page_or_a_redirect_is_an_error() {
    let to_gone = format!(r#"<link rel="canonical" href="{POST}"><a href="moved/">Moved</a>"#);
    let moved = titled(
        "Moved",
        r#"<link rel="canonical" href="https://www.example.com/blog/old/">"#,
    );
    let old = Answer::new(301, Vec::new()).with_header("location", "/blog/moved/");
    let crawl = crawled(&[
        (HOME, titled("Home", &to_gone)),
        ("https://www.example.com/blog/moved/", moved),
        ("https://www.example.com/blog/old/", old),
    ])
    .await;
    assert_eq!(
        only(&crawl, &[Check::CanonicalBroken, Check::CanonicalRedirect]),
        [
            (Check::CanonicalBroken, HOME.to_owned(), POST.to_owned()),
            (
                Check::CanonicalRedirect,
                "https://www.example.com/blog/moved/".to_owned(),
                "https://www.example.com/blog/old/".to_owned()
            ),
        ]
    );
}

#[tokio::test]
async fn translations_that_name_each_other_and_themselves_pass() {
    let both = format!("{}{}", alternate("en", POST), alternate("de", GERMAN));
    let crawl = crawled(&[
        (
            HOME,
            titled("Home", &format!(r#"<a href="{POST}">Post</a>"#)),
        ),
        (POST, titled("Post", &both)),
        (GERMAN, titled("Beitrag", &both)),
    ])
    .await;
    let localization = [
        Check::HreflangNoSelf,
        Check::HreflangBroken,
        Check::HreflangRedirect,
        Check::HreflangNoReturn,
    ];
    assert_eq!(only(&crawl, &localization), []);
}

#[tokio::test]
async fn a_one_way_broken_or_selfless_translation_is_named() {
    let english = format!(
        "{}{}{}",
        alternate("en", POST),
        alternate("de", GERMAN),
        alternate("fr", "https://www.example.com/blog/fr/post/")
    );
    let crawl = crawled(&[
        (
            HOME,
            titled("Home", &format!(r#"<a href="{POST}">Post</a>"#)),
        ),
        (POST, titled("Post", &english)),
        (GERMAN, titled("Beitrag", "")),
    ])
    .await;
    let localization = [
        Check::HreflangNoSelf,
        Check::HreflangBroken,
        Check::HreflangNoReturn,
    ];
    assert_eq!(
        only(&crawl, &localization),
        [
            (
                Check::HreflangBroken,
                POST.to_owned(),
                "https://www.example.com/blog/fr/post/".to_owned()
            ),
            (Check::HreflangNoReturn, POST.to_owned(), GERMAN.to_owned()),
        ]
    );
    let selfless = crawled(&[(HOME, titled("Home", &alternate("de", GERMAN)))]).await;
    assert_eq!(
        only(&selfless, &[Check::HreflangNoSelf]),
        [(Check::HreflangNoSelf, HOME.to_owned(), String::new())]
    );
}

#[tokio::test]
async fn listed_pages_sharing_a_title_or_description_are_each_named() {
    let crawl = crawled(&[
        (
            HOME,
            titled(
                "Home",
                r#"<a href="a/">A</a><a href="b/">B</a><a href="c/">C</a>"#,
            ),
        ),
        ("https://www.example.com/blog/a/", titled("Same", "")),
        ("https://www.example.com/blog/b/", titled("Same", "")),
        (
            "https://www.example.com/blog/c/",
            titled("Same", r#"<meta name="robots" content="noindex">"#),
        ),
    ])
    .await;
    let a = "https://www.example.com/blog/a/";
    let b = "https://www.example.com/blog/b/";
    assert_eq!(
        only(
            &crawl,
            &[Check::TitleDuplicate, Check::DescriptionDuplicate]
        ),
        [
            (Check::TitleDuplicate, a.to_owned(), b.to_owned()),
            (Check::TitleDuplicate, b.to_owned(), a.to_owned()),
            (Check::DescriptionDuplicate, a.to_owned(), b.to_owned()),
            (Check::DescriptionDuplicate, b.to_owned(), a.to_owned()),
        ]
    );
}
