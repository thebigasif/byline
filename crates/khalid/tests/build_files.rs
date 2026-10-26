//! `khalid build` writes the site's files beside its pages: the sitemap
//! index and sitemaps, `robots.txt` and `llms.txt` (KH-SEO-5, KH-SEO-7).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

#[cfg(test)]
fn built() -> TempDir {
    let out = tempfile::tempdir().unwrap();
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", demo.to_str().unwrap(), "build", "--out"])
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

#[test]
fn the_sitemap_index() {
    insta::assert_snapshot!(read(&built(), "sitemap.xml"));
}

#[test]
fn the_sitemap_of_english_posts() {
    insta::assert_snapshot!(read(&built(), "sitemaps/posts-en-1.xml"));
}

#[test]
fn robots_txt() {
    insta::assert_snapshot!(read(&built(), "robots.txt"));
}

/// The snapshot leaves out the German lines, which the English spelling
/// check in `just check` would read as typos; the next test checks them.
#[test]
fn llms_txt_without_its_german_lines() {
    let text = read(&built(), "llms.txt");
    let english: Vec<&str> = text
        .lines()
        .filter(|line| !line.contains("/blog/de/"))
        .collect();
    insta::assert_snapshot!(english.join("\n"));
}

#[test]
fn llms_txt_lists_the_german_posts_and_pages() {
    let text = read(&built(), "llms.txt");
    for path in ["de/warum-interne-links-wichtig-sind/", "de/ueber-uns/"] {
        let link = format!("](https://www.example.com/blog/{path}): ");
        assert!(text.contains(&link), "{link}\n{text}");
    }
}

#[test]
fn every_sitemap_the_index_lists_is_written() {
    let out = built();
    let index = read(&out, "sitemap.xml");
    let listed: Vec<&str> = index
        .split("<loc>https://www.example.com/blog/")
        .skip(1)
        .filter_map(|rest| rest.split("</loc>").next())
        .collect();
    assert!(listed.len() >= 4, "{index}");
    for rel in listed {
        assert!(read(&out, rel).contains("<urlset"), "{rel}");
    }
}

#[test]
fn the_sitemaps_list_no_page_kept_out_of_search() {
    let out = built();
    for rel in ["sitemaps/pages-en-1.xml", "sitemaps/pages-de-1.xml"] {
        assert!(!read(&out, rel).contains("404"), "{rel}");
    }
}
