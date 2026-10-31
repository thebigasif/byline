//! The page-weight meter: how many bytes each page and the files it loads
//! weigh, by kind, heaviest page first.

pub mod support;

use khalid_audit::{Answer, PageWeight, WeightReport};
use support::{HOME, crawled, html, page};

/// A 200 answer of `bytes` bytes with the content type `kind`.
fn file(kind: &str, bytes: usize) -> Answer {
    Answer::new(200, vec![b'x'; bytes]).with_header("content-type", kind)
}

/// Each page's address after the home page, with its weights by kind:
/// HTML, styles, scripts, images, other, total and the files not weighed.
fn rows(report: &WeightReport) -> Vec<(&str, [usize; 7])> {
    report
        .pages()
        .iter()
        .map(|page: &PageWeight| {
            (
                page.url().as_str().trim_start_matches(HOME),
                [
                    page.html(),
                    page.styles(),
                    page.scripts(),
                    page.images(),
                    page.other(),
                    page.total(),
                    page.unweighed(),
                ],
            )
        })
        .collect()
}

#[tokio::test]
async fn a_page_weighs_its_html_and_each_file_it_loads_once() {
    let home = page(
        r#"<link rel="stylesheet" href="style.css"><script src="app.js"></script>"#,
        concat!(
            r#"<img src="a.webp" alt="A"><img src="a.webp" alt="A again">"#,
            r#"<img src="b.gif" alt="B"><img src="gone.png" alt="Gone">"#,
            r#"<img src="https://cdn.example.org/c.png" alt="C">"#,
        ),
    );
    let html_bytes = home.body().len();
    let crawl = crawled(&[
        (HOME, home),
        (
            "https://www.example.com/blog/style.css",
            file("text/css; charset=utf-8", 1_000),
        ),
        (
            "https://www.example.com/blog/app.js",
            file("text/javascript", 300),
        ),
        (
            "https://www.example.com/blog/a.webp",
            file("image/webp", 20_000),
        ),
        (
            "https://www.example.com/blog/b.gif",
            file("application/octet-stream", 50),
        ),
    ])
    .await;
    let report = WeightReport::of(&crawl);
    let total = html_bytes + 1_000 + 300 + 20_000 + 50;
    // The image shown twice downloads once. The missing image and the one
    // on another site cannot be weighed, so they are counted apart.
    assert_eq!(
        rows(&report),
        [("", [html_bytes, 1_000, 300, 20_000, 50, total, 2])]
    );
}

#[tokio::test]
async fn a_file_behind_a_redirect_weighs_what_it_leads_to() {
    let crawl = crawled(&[
        (HOME, html(r#"<img src="old.png" alt="Old">"#)),
        (
            "https://www.example.com/blog/old.png",
            Answer::new(301, Vec::new()).with_header("location", "new.png"),
        ),
        (
            "https://www.example.com/blog/new.png",
            file("image/png", 4_000),
        ),
    ])
    .await;
    let report = WeightReport::of(&crawl);
    let weights = rows(&report);
    let images: Vec<(usize, usize)> = weights
        .iter()
        .map(|(_, weight)| (weight[3], weight[6]))
        .collect();
    assert_eq!(images, [(4_000, 0)]);
}

#[tokio::test]
async fn a_redirect_loop_is_not_weighed_and_does_not_hang() {
    let crawl = crawled(&[
        (HOME, html(r#"<img src="a.png" alt="A">"#)),
        (
            "https://www.example.com/blog/a.png",
            Answer::new(302, Vec::new()).with_header("location", "b.png"),
        ),
        (
            "https://www.example.com/blog/b.png",
            Answer::new(302, Vec::new()).with_header("location", "a.png"),
        ),
    ])
    .await;
    let report = WeightReport::of(&crawl);
    let unweighed: Vec<usize> = report.pages().iter().map(PageWeight::unweighed).collect();
    assert_eq!(unweighed, [1]);
}

#[tokio::test]
async fn pages_come_heaviest_first_and_each_counts_a_shared_file() {
    let crawl = crawled(&[
        (
            HOME,
            page(
                r#"<link rel="stylesheet" href="style.css">"#,
                r#"<a href="light/">Light</a><a href="heavy/">Heavy</a>"#,
            ),
        ),
        (
            "https://www.example.com/blog/light/",
            page(r#"<link rel="stylesheet" href="../style.css">"#, ""),
        ),
        (
            "https://www.example.com/blog/heavy/",
            page(
                r#"<link rel="stylesheet" href="../style.css">"#,
                r#"<img src="../big.avif" alt="Big">"#,
            ),
        ),
        (
            "https://www.example.com/blog/style.css",
            file("text/css", 2_000),
        ),
        (
            "https://www.example.com/blog/big.avif",
            file("image/avif", 90_000),
        ),
    ])
    .await;
    let report = WeightReport::of(&crawl);
    let order: Vec<&str> = rows(&report).iter().map(|(url, _)| *url).collect();
    assert_eq!(order, ["heavy/", "", "light/"]);
    let styles: Vec<usize> = report.pages().iter().map(PageWeight::styles).collect();
    assert_eq!(styles, [2_000, 2_000, 2_000]);
    assert_eq!(
        report.heaviest().map(|page| page.url().as_str()),
        Some("https://www.example.com/blog/heavy/")
    );
}

#[tokio::test]
async fn the_csv_lists_every_page_with_its_weights() {
    let crawl = crawled(&[
        (HOME, html(r#"<img src="a,b.png" alt="A">"#)),
        (
            "https://www.example.com/blog/a,b.png",
            file("image/png", 10),
        ),
    ])
    .await;
    let report = WeightReport::of(&crawl);
    let html_bytes = rows(&report).first().map_or(0, |(_, weight)| weight[0]);
    let csv = report.csv();
    let mut lines = csv.lines();
    assert_eq!(
        lines.next(),
        Some("url,total,html,styles,scripts,images,other,unweighed")
    );
    let total = html_bytes + 10;
    assert_eq!(
        lines.next(),
        Some(format!("{HOME},{total},{html_bytes},0,0,10,0,0").as_str())
    );
    assert_eq!(lines.next(), None);
}

#[tokio::test]
async fn a_crawl_with_no_page_has_no_heaviest_page() {
    let crawl = crawled(&[]).await;
    let report = WeightReport::of(&crawl);
    assert_eq!(report.pages(), []);
    assert!(report.heaviest().is_none());
}
