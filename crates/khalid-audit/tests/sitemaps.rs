//! The files a crawl reads before the pages: `robots.txt` at the root of
//! the host, the blog's `llms.txt`, and the sitemaps.

pub mod support;

use khalid_audit::Answer;
use support::{HOME, crawled, html};

#[cfg(test)]
fn text(body: &str) -> Answer {
    Answer::new(200, body.as_bytes().to_vec()).with_header("content-type", "text/plain")
}

#[cfg(test)]
fn urlset(pages: &[&str]) -> Answer {
    let urls = pages.join("</loc></url><url><loc>");
    let urls = format!("<url><loc>{urls}</loc></url>");
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">{urls}</urlset>"#
    );
    Answer::new(200, xml.into_bytes()).with_header("content-type", "application/xml")
}

#[tokio::test]
async fn reads_the_sitemaps_robots_names_and_visits_pages_only_they_list() {
    let index = r#"<?xml version="1.0"?><sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
<sitemap><loc>https://www.example.com/blog/sitemaps/posts.xml</loc></sitemap></sitemapindex>"#;
    let crawl = crawled(&[
        (
            "https://www.example.com/robots.txt",
            text("User-agent: *\nAllow: /\nSitemap: https://www.example.com/blog/sitemap.xml\nSitemap: https://other.example.org/sitemap.xml\n"),
        ),
        (
            "https://www.example.com/blog/sitemap.xml",
            Answer::new(200, index.as_bytes().to_vec()),
        ),
        (
            "https://www.example.com/blog/sitemaps/posts.xml",
            urlset(&[HOME, "https://www.example.com/blog/hidden/"]),
        ),
        ("https://www.example.com/blog/llms.txt", text("# Field Notes on SEO\n")),
        (HOME, html("")),
        ("https://www.example.com/blog/hidden/", html("")),
    ])
    .await;
    let files = crawl.files();
    assert!(
        files
            .robots()
            .is_some_and(|robots| robots.contains("Sitemap:"))
    );
    assert_eq!(files.llms(), Some("# Field Notes on SEO\n"));
    let sitemaps: Vec<(&str, u16, usize)> = files
        .sitemaps()
        .iter()
        .map(|sitemap| {
            (
                sitemap.url().as_str(),
                sitemap.status(),
                sitemap.pages().len(),
            )
        })
        .collect();
    // The sitemap on another host is left out.
    assert_eq!(
        sitemaps,
        [
            ("https://www.example.com/blog/sitemap.xml", 200, 0),
            ("https://www.example.com/blog/sitemaps/posts.xml", 200, 2),
        ]
    );
    let hidden = crawl.visit("https://www.example.com/blog/hidden/").unwrap();
    assert_eq!(hidden.depth(), None);
    assert_eq!(crawl.visit(HOME).unwrap().depth(), Some(0));
}

#[tokio::test]
async fn without_robots_the_blogs_own_sitemap_is_read() {
    let crawl = crawled(&[
        ("https://www.example.com/blog/sitemap.xml", urlset(&[HOME])),
        (HOME, html("")),
    ])
    .await;
    let files = crawl.files();
    assert_eq!(files.robots(), None);
    assert_eq!(files.llms(), None);
    assert_eq!(files.listed().count(), 1);
}

#[tokio::test]
async fn a_missing_sitemap_is_kept_with_its_status_and_lists_nothing() {
    let crawl = crawled(&[(HOME, html(""))]).await;
    let sitemaps = crawl.files().sitemaps();
    assert_eq!(sitemaps.len(), 1);
    assert_eq!(
        sitemaps.first().map(khalid_audit::Sitemap::status),
        Some(404)
    );
    assert_eq!(crawl.files().listed().count(), 0);
}
