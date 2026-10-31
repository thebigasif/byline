//! `khalid build` writes the news sitemap of a news collection, lists it in
//! the sitemap index, and types its posts `NewsArticle` (KH-SEO-5, G16).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::TimeDelta;
use khalid_core::{Clock, SystemClock};
use tempfile::TempDir;

/// `rel` under `base`.
#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// Copies the folder `from` into `to`.
#[cfg(test)]
fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap().map(Result::unwrap) {
        let target = at(to, &entry.file_name().to_string_lossy());
        if entry.path().is_dir() {
            copy(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Yesterday, by the clock `khalid build` reads, so the post is inside the
/// two days a news sitemap covers whenever the test runs.
#[cfg(test)]
fn yesterday() -> String {
    (SystemClock.now() - TimeDelta::days(1))
        .format("%Y-%m-%d")
        .to_string()
}

/// The demo site with a `news` collection holding "What a sitemap does",
/// dated `date`.
#[cfg(test)]
fn news_site(date: &str) -> TempDir {
    let site = tempfile::tempdir().unwrap();
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    copy(&demo, site.path());
    let routes = "[collections.news]\npermalink = \"/news/{slug}/\"\nnews = true\n";
    fs::write(at(site.path(), "routes.toml"), routes).unwrap();
    let old = at(
        site.path(),
        "content/en/posts/2026-09-28-what-a-sitemap-does.md",
    );
    let text = fs::read_to_string(&old).unwrap();
    fs::remove_file(&old).unwrap();
    let news = at(site.path(), "content/en/news");
    fs::create_dir_all(&news).unwrap();
    let text = text.replace("date: 2026-09-28", &format!("date: {date}"));
    let file = at(&news, &format!("{date}-what-a-sitemap-does.md"));
    fs::write(file, text).unwrap();
    site
}

/// `site` built into a new folder.
#[cfg(test)]
fn build(site: &Path) -> TempDir {
    let out = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "build", "--out"])
        .arg(out.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    out
}

#[cfg(test)]
fn read(out: &TempDir, rel: &str) -> String {
    fs::read_to_string(at(out.path(), rel)).unwrap_or_else(|error| panic!("{rel}: {error}"))
}

/// The snapshot is the news sitemap; the index lists it, and only the news
/// post is a `NewsArticle`.
#[test]
fn the_news_sitemap_lists_the_fresh_post_of_a_news_collection() {
    let date = yesterday();
    let out = build(news_site(&date).path());
    let news = read(&out, "sitemaps/news.xml").replace(&date, "YESTERDAY");
    insta::assert_snapshot!(news);
    let index = read(&out, "sitemap.xml");
    let listed = "<loc>https://www.example.com/blog/sitemaps/news.xml</loc>";
    assert!(index.contains(listed), "{index}");
    let page = read(&out, "news/what-a-sitemap-does/index.html");
    assert!(page.contains("\"@type\":\"NewsArticle\""), "{page}");
    let other = read(&out, "why-internal-links-matter/index.html");
    assert!(!other.contains("NewsArticle"), "{other}");
}

#[test]
fn a_news_post_older_than_two_days_leaves_the_news_sitemap_empty() {
    let out = build(news_site("2026-09-28").path());
    let news = read(&out, "sitemaps/news.xml");
    assert!(!news.contains("<url>"), "{news}");
}

#[test]
fn a_site_without_news_writes_no_news_sitemap() {
    let out = build(&at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    ));
    assert!(!at(out.path(), "sitemaps/news.xml").exists());
    assert!(!read(&out, "sitemap.xml").contains("news.xml"));
}

#[test]
fn each_image_the_sitemap_lists_is_a_file_the_build_wrote() {
    let out = build(news_site(&yesterday()).path());
    let posts = read(&out, "sitemaps/posts-en-1.xml");
    let images: Vec<&str> = posts
        .lines()
        .filter_map(|line| line.trim().strip_prefix("<image:loc>"))
        .filter_map(|line| line.strip_suffix("</image:loc>"))
        .collect();
    assert!(!images.is_empty(), "{posts}");
    for image in images {
        let rel = image.trim_start_matches("https://www.example.com/blog/");
        assert!(at(out.path(), rel).is_file(), "{image} was not written");
    }
}
