//! The checks that need the whole crawl: broken pages and links,
//! redirects, broken files, click depth and inbound links.

pub mod support;

use khalid_audit::{Answer, Check, Crawl, Thresholds, audit};
use support::{HOME, crawled, html};

/// The issues of `checks` in `crawl`, as check, address and detail.
#[cfg(test)]
fn only(crawl: &Crawl, checks: &[Check]) -> Vec<(Check, String, String)> {
    audit(crawl, Thresholds::default())
        .iter()
        .filter(|issue| checks.contains(&issue.check()))
        .map(|issue| {
            let url = issue.url().as_str().trim_start_matches(HOME).to_owned();
            (issue.check(), url, issue.detail().to_owned())
        })
        .collect()
}

#[cfg(test)]
fn moved(to: &str) -> Answer {
    Answer::new(301, Vec::new()).with_header("location", to)
}

#[tokio::test]
async fn a_link_to_a_missing_page_names_both_pages() {
    let crawl = crawled(&[(HOME, html(r#"<a href="gone/">Gone</a>"#))]).await;
    assert_eq!(
        only(&crawl, &[Check::Page4xx, Check::BrokenLink]),
        [
            (Check::Page4xx, "gone/".to_owned(), "404".to_owned()),
            (
                Check::BrokenLink,
                String::new(),
                "https://www.example.com/blog/gone/".to_owned()
            ),
        ]
    );
}

#[tokio::test]
async fn a_server_fault_on_a_linked_page_is_named() {
    let crawl = crawled(&[
        (HOME, html(r#"<a href="fails/">Fails</a>"#)),
        (
            "https://www.example.com/blog/fails/",
            Answer::new(503, Vec::new()),
        ),
    ])
    .await;
    assert_eq!(
        only(&crawl, &[Check::Page5xx]),
        [(Check::Page5xx, "fails/".to_owned(), "503".to_owned())]
    );
}

#[tokio::test]
async fn redirects_chains_and_loops_are_told_apart() {
    let crawl = crawled(&[
        (
            HOME,
            html(r#"<a href="one/">One</a><a href="a/">A</a><a href="loop/">Loop</a>"#),
        ),
        ("https://www.example.com/blog/one/", moved("/blog/two/")),
        ("https://www.example.com/blog/two/", html("")),
        ("https://www.example.com/blog/a/", moved("/blog/b/")),
        ("https://www.example.com/blog/b/", moved("/blog/two/")),
        ("https://www.example.com/blog/loop/", moved("/blog/loop/")),
    ])
    .await;
    let checks = [
        Check::LinksToRedirect,
        Check::RedirectChain,
        Check::RedirectLoop,
    ];
    assert_eq!(
        only(&crawl, &checks),
        [
            (
                Check::LinksToRedirect,
                String::new(),
                "https://www.example.com/blog/a/".to_owned()
            ),
            (
                Check::LinksToRedirect,
                String::new(),
                "https://www.example.com/blog/loop/".to_owned()
            ),
            (
                Check::LinksToRedirect,
                String::new(),
                "https://www.example.com/blog/one/".to_owned()
            ),
            (Check::RedirectChain, "a/".to_owned(), "2 hops".to_owned()),
            (Check::RedirectLoop, "loop/".to_owned(), String::new()),
        ]
    );
}

#[tokio::test]
async fn a_missing_image_or_stylesheet_is_named_on_its_page() {
    let crawl = crawled(&[(
        HOME,
        html(r#"<img src="a.png" alt=""><link rel="stylesheet" href="site.css">"#),
    )])
    .await;
    assert_eq!(
        only(
            &crawl,
            &[Check::ImageBroken, Check::ResourceBroken, Check::Page4xx]
        ),
        [
            (
                Check::ImageBroken,
                String::new(),
                "https://www.example.com/blog/a.png".to_owned()
            ),
            (
                Check::ResourceBroken,
                String::new(),
                "https://www.example.com/blog/site.css".to_owned()
            ),
        ]
    );
}

#[tokio::test]
async fn a_page_four_clicks_deep_is_too_deep() {
    let crawl = crawled(&[
        (HOME, html(r#"<a href="1/">1</a>"#)),
        (
            "https://www.example.com/blog/1/",
            html(r#"<a href="../2/">2</a>"#),
        ),
        (
            "https://www.example.com/blog/2/",
            html(r#"<a href="../3/">3</a>"#),
        ),
        (
            "https://www.example.com/blog/3/",
            html(r#"<a href="../4/">4</a>"#),
        ),
        ("https://www.example.com/blog/4/", html("")),
    ])
    .await;
    assert_eq!(
        only(&crawl, &[Check::TooDeep]),
        [(Check::TooDeep, "4/".to_owned(), "4 clicks".to_owned())]
    );
}

#[tokio::test]
async fn a_page_with_one_followed_link_to_it_is_named() {
    let crawl = crawled(&[
        (
            HOME,
            html(r#"<a href="one/">One</a><a href="two/">Two</a><a href="two/" rel="nofollow">Two</a>"#),
        ),
        (
            "https://www.example.com/blog/one/",
            html(r#"<a href="../two/">Two</a><a href="./">Itself</a>"#),
        ),
        ("https://www.example.com/blog/two/", html("")),
    ])
    .await;
    assert_eq!(
        only(&crawl, &[Check::OneInboundLink]),
        [(Check::OneInboundLink, "one/".to_owned(), String::new())]
    );
}
