//! The checks on the sitemaps and what they list, `robots.txt`,
//! `llms.txt`, and pages only a sitemap leads to.

pub mod support;

use khalid_audit::{Answer, Check, Crawl, Thresholds, audit};
use support::{HOME, crawled, html, page};

const ROBOTS: &str = "https://www.example.com/robots.txt";
const SITEMAP: &str = "https://www.example.com/blog/sitemap.xml";
const LLMS: &str = "https://www.example.com/blog/llms.txt";

/// The checks this file is about.
const CHECKS: [Check; 10] = [
    Check::RobotsMissing,
    Check::SitemapMissing,
    Check::SitemapBroken,
    Check::SitemapBrokenPage,
    Check::SitemapRedirect,
    Check::SitemapNoindex,
    Check::SitemapNonCanonical,
    Check::Orphan,
    Check::LlmsMissing,
    Check::LlmsFormat,
];

#[cfg(test)]
fn found(crawl: &Crawl) -> Vec<(Check, String)> {
    audit(crawl, Thresholds::default())
        .iter()
        .filter(|issue| CHECKS.contains(&issue.check()))
        .map(|issue| (issue.check(), issue.url().as_str().to_owned()))
        .collect()
}

#[cfg(test)]
fn text(body: &str) -> Answer {
    Answer::new(200, body.as_bytes().to_vec())
}

#[cfg(test)]
fn urlset(pages: &[&str]) -> Answer {
    let urls = pages.join("</loc></url><url><loc>");
    let xml = format!(
        r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>{urls}</loc></url></urlset>"#
    );
    Answer::new(200, xml.into_bytes())
}

#[tokio::test]
async fn a_blog_with_its_files_in_place_has_no_issues() {
    let crawl = crawled(&[
        (
            ROBOTS,
            text(&format!("User-agent: *\nSitemap: {SITEMAP}\n")),
        ),
        (SITEMAP, urlset(&[HOME])),
        (LLMS, text("# Field Notes on SEO\n")),
        (HOME, html("")),
    ])
    .await;
    assert_eq!(found(&crawl), []);
}

#[tokio::test]
async fn missing_files_are_named_on_the_home_page_or_the_sitemap() {
    let crawl = crawled(&[(HOME, html(""))]).await;
    assert_eq!(
        found(&crawl),
        [
            (Check::RobotsMissing, HOME.to_owned()),
            (Check::SitemapMissing, SITEMAP.to_owned()),
            (Check::LlmsMissing, HOME.to_owned()),
        ]
    );
}

#[tokio::test]
async fn a_sitemap_robots_names_that_fails_and_a_bad_llms_file_are_named() {
    let gone = "https://www.example.com/blog/gone.xml";
    let crawl = crawled(&[
        (ROBOTS, text(&format!("Sitemap: {gone}\n"))),
        (LLMS, text("Field Notes on SEO\n")),
        (HOME, html("")),
    ])
    .await;
    assert_eq!(
        found(&crawl),
        [
            (Check::SitemapBroken, gone.to_owned()),
            (Check::LlmsFormat, HOME.to_owned()),
        ]
    );
}

#[tokio::test]
async fn pages_a_sitemap_should_not_list_and_orphans_are_named() {
    let gone = "https://www.example.com/blog/gone/";
    let moved = "https://www.example.com/blog/moved/";
    let hidden = "https://www.example.com/blog/hidden/";
    let copy = "https://www.example.com/blog/copy/";
    let orphan = "https://www.example.com/blog/orphan/";
    let crawl = crawled(&[
        (ROBOTS, text(&format!("Sitemap: {SITEMAP}\n"))),
        (LLMS, text("# Field Notes on SEO\n")),
        (SITEMAP, urlset(&[HOME, gone, moved, hidden, copy, orphan])),
        (
            HOME,
            html(r#"<a href="hidden/">Hidden</a><a href="copy/">Copy</a>"#),
        ),
        (
            moved,
            Answer::new(301, Vec::new()).with_header("location", "/blog/"),
        ),
        (
            hidden,
            page(r#"<meta name="robots" content="noindex">"#, ""),
        ),
        (
            copy,
            page(&format!(r#"<link rel="canonical" href="{HOME}">"#), ""),
        ),
        (orphan, html("")),
    ])
    .await;
    assert_eq!(
        found(&crawl),
        [
            (Check::SitemapBrokenPage, gone.to_owned()),
            (Check::SitemapRedirect, moved.to_owned()),
            (Check::SitemapNoindex, hidden.to_owned()),
            (Check::SitemapNonCanonical, copy.to_owned()),
            (Check::Orphan, orphan.to_owned()),
        ]
    );
}
