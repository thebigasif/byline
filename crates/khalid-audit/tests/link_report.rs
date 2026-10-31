//! The internal linking report: click depth, links in and out, link rank
//! and distribution.

pub mod support;

use khalid_audit::LinkReport;
use support::{HOME, crawled, html};

#[tokio::test]
async fn each_page_gets_its_depth_links_and_rank() {
    let crawl = crawled(&[
        (
            HOME,
            html(r#"<a href="pillar/">Pillar</a><a href="one/">One</a><a href="https://serpdad.com/">SerpDad</a>"#),
        ),
        (
            "https://www.example.com/blog/pillar/",
            html(r#"<a href="../one/">One</a><a href="../">Home</a>"#),
        ),
        (
            "https://www.example.com/blog/one/",
            html(r#"<a href="../pillar/">Pillar</a><a href="../pillar/" rel="nofollow">Again</a>"#),
        ),
    ])
    .await;
    let report = LinkReport::of(&crawl);
    let rows: Vec<(&str, Option<u32>, usize, usize, usize)> = report
        .pages()
        .iter()
        .map(|page| {
            let url = page.url().as_str().trim_start_matches(HOME);
            (
                url,
                page.depth(),
                page.inbound(),
                page.outbound(),
                page.external(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("", Some(0), 1, 2, 1),
            ("pillar/", Some(1), 2, 2, 0),
            ("one/", Some(1), 2, 1, 0),
        ]
    );
    // The pillar post gets the most links and the most rank.
    let ranks: Vec<u8> = report
        .pages()
        .iter()
        .map(khalid_audit::PageLinks::rank)
        .collect();
    assert_eq!(ranks.get(1), Some(&100));
    assert!(
        ranks.iter().all(|rank| *rank > 0 && *rank <= 100),
        "{ranks:?}"
    );
    assert!(ranks.first() < ranks.get(1));
}

#[tokio::test]
async fn the_distribution_counts_pages_per_click_and_the_csv_lists_them() {
    let crawl = crawled(&[
        (HOME, html(r#"<a href="a/">A</a><a href="b/">B</a>"#)),
        ("https://www.example.com/blog/a/", html("")),
        ("https://www.example.com/blog/b/", html("")),
    ])
    .await;
    let report = LinkReport::of(&crawl);
    let distribution: Vec<(Option<u32>, usize)> = report.distribution().into_iter().collect();
    assert_eq!(distribution, [(Some(0), 1), (Some(1), 2)]);
    let csv = report.csv();
    let mut lines = csv.lines();
    assert_eq!(
        lines.next(),
        Some("url,depth,inbound,outbound,external,rank")
    );
    // No page links to home, so it keeps only its 15 parts, while each
    // leaf keeps 15 and gets half of the 85% home passes on, 21.375 in
    // all: home ranks 15 / 21.375, 70 out of 100.
    assert_eq!(
        lines.next(),
        Some("https://www.example.com/blog/,0,0,2,0,70")
    );
    assert_eq!(lines.count(), 2);
}

#[tokio::test]
async fn a_crawl_without_pages_has_an_empty_report() {
    let crawl = crawled(&[]).await;
    let report = LinkReport::of(&crawl);
    assert_eq!(report.pages().len(), 0);
    assert_eq!(report.csv(), "url,depth,inbound,outbound,external,rank\n");
}
