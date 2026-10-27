//! The crawl: from the home page, following links the way a search engine
//! does, staying inside the blog, and reading each page once.

pub mod support;

use khalid_audit::{Answer, Crawl, Limit, Outcome, Source, Unreached, crawl};
use khalid_core::AbsoluteUrl;
use support::{Memory, crawled, html, url};

fn depths(crawl: &Crawl) -> Vec<(&str, u32)> {
    crawl
        .visits()
        .iter()
        .filter_map(|visit| Some((visit.url().as_str(), visit.depth()?)))
        .collect()
}

#[tokio::test]
async fn follows_links_from_the_home_page_and_counts_clicks() {
    let crawl = crawled(&[
        (
            "https://www.example.com/blog/",
            html(r#"<a href="/blog/first/">First</a> <a href="second/">Second</a>"#),
        ),
        (
            "https://www.example.com/blog/first/",
            html(r#"<a href="../third/">Third</a> <a href="/blog/">Home</a>"#),
        ),
        ("https://www.example.com/blog/second/", html("")),
        ("https://www.example.com/blog/third/", html("")),
    ])
    .await;
    assert_eq!(
        depths(&crawl),
        [
            ("https://www.example.com/blog/", 0),
            ("https://www.example.com/blog/first/", 1),
            ("https://www.example.com/blog/second/", 1),
            ("https://www.example.com/blog/third/", 2),
        ]
    );
}

#[tokio::test]
async fn checks_links_outside_the_blog_but_never_follows_them() {
    let crawl = crawled(&[
        (
            "https://www.example.com/blog/",
            html(
                r#"<a href="https://www.example.com/shop/">Shop</a> <a href="https://serpdad.com/">SerpDad</a> <a href="mailto:a@example.com">Mail</a>"#,
            ),
        ),
        (
            "https://www.example.com/shop/",
            html(r#"<a href="https://www.example.com/blog/hidden/">Hidden</a>"#),
        ),
    ])
    .await;
    assert_eq!(depths(&crawl), [("https://www.example.com/blog/", 0)]);
    let outside: Vec<&str> = crawl.outside().iter().map(AbsoluteUrl::as_str).collect();
    assert_eq!(
        outside,
        ["https://serpdad.com/", "https://www.example.com/shop/"]
    );
}

#[tokio::test]
async fn records_a_redirect_and_follows_it_at_the_same_depth() {
    let crawl = crawled(&[
        (
            "https://www.example.com/blog/",
            html(r#"<a href="/blog/old/">Old</a> <a href="/blog/gone/">Gone</a>"#),
        ),
        (
            "https://www.example.com/blog/old/",
            Answer::new(301, Vec::new()).with_header("location", "/blog/new/"),
        ),
        ("https://www.example.com/blog/new/", html("")),
    ])
    .await;
    assert_eq!(
        depths(&crawl),
        [
            ("https://www.example.com/blog/", 0),
            ("https://www.example.com/blog/old/", 1),
            ("https://www.example.com/blog/gone/", 1),
            ("https://www.example.com/blog/new/", 1),
        ]
    );
    let old = crawl.visit("https://www.example.com/blog/old/").unwrap();
    assert_eq!(
        old.outcome(),
        &Outcome::Redirect {
            status: 301,
            to: url("https://www.example.com/blog/new/")
        }
    );
    let gone = crawl.visit("https://www.example.com/blog/gone/").unwrap();
    assert_eq!(gone.outcome(), &Outcome::Status(404));
}

#[tokio::test]
async fn asks_for_the_files_a_page_loads_but_reads_no_page_there() {
    let css = Answer::new(200, b"p{}".to_vec()).with_header("content-type", "text/css");
    let crawl = crawled(&[
        (
            "https://www.example.com/blog/",
            html(r#"<link rel="stylesheet" href="/blog/site.css"><img src="/blog/a.png" alt="">"#),
        ),
        ("https://www.example.com/blog/site.css", css),
    ])
    .await;
    let css = crawl
        .visit("https://www.example.com/blog/site.css")
        .unwrap();
    assert_eq!(css.outcome(), &Outcome::Status(200));
    assert_eq!(css.bytes(), 3);
    assert!(css.page().is_none());
    let image = crawl.visit("https://www.example.com/blog/a.png").unwrap();
    assert!(image.outcome().is_broken());
    let home = crawl.visit("https://www.example.com/blog/").unwrap();
    assert_eq!(
        home.header("Content-Type"),
        Some("text/html; charset=utf-8")
    );
}

#[tokio::test]
async fn stops_at_the_page_limit_and_says_so() {
    let crawl = crawl(
        &Memory::new(&[(
            "https://www.example.com/blog/",
            html(r#"<a href="/blog/a/">A</a> <a href="/blog/b/">B</a>"#),
        )]),
        &url("https://www.example.com/blog/"),
        Limit::pages(2),
    )
    .await;
    assert_eq!(crawl.visits().len(), 2);
    assert!(crawl.stopped_at_limit());
}

#[tokio::test]
async fn an_address_that_cannot_be_reached_is_kept_with_why() {
    struct Down;
    impl Source for Down {
        fn fetch(&self, _: &AbsoluteUrl) -> impl Future<Output = Result<Answer, Unreached>> {
            std::future::ready(Err(Unreached::TimedOut))
        }
    }
    let crawl = crawl(
        &Down,
        &url("https://www.example.com/blog/"),
        Limit::default(),
    )
    .await;
    assert_eq!(
        crawl.visits().first().unwrap().outcome(),
        &Outcome::Unreached(Unreached::TimedOut)
    );
}
