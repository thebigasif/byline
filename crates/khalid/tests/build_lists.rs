//! `khalid build` writes the list pages of audit brief G12: the archive of
//! every post, twenty to a page, and the page of every author.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const SITE_TOML: &str = r#"name = "Field Notes on SEO"
address = "https://www.example.com"
mount = "/blog/"
mode = "static"
default_language = "en"
languages = ["en"]
"#;

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// Writes `text` to `rel` inside `site`, making its folders first.
#[cfg(test)]
fn put(site: &Path, rel: &str, text: &str) {
    let file = at(site, rel);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, text).unwrap();
}

/// A site with `posts` posts by Asif, one a day from 1 September 2026,
/// and `extra` added to `site.toml`.
#[cfg(test)]
fn site(posts: u32, extra: &str) -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    put(temp.path(), "site.toml", &format!("{SITE_TOML}{extra}"));
    put(
        temp.path(),
        "content/en/authors/asif.md",
        "---\nname: Asif\nslug: asif\nrole: Founder of SerpDad\n---\n\nAsif writes about links.\n",
    );
    for day in 1..=posts {
        let text = format!(
            "---\ntitle: Post {day}\nslug: post-{day}\ndate: 2026-09-{day:02}\nauthor: asif\n---\n\n\
             Post {day} links to [the home page](/blog/).\n"
        );
        put(
            temp.path(),
            &format!("content/en/posts/post-{day}.md"),
            &text,
        );
    }
    temp
}

#[cfg(test)]
fn build(site: &TempDir) -> String {
    let output: Output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site.path())
        .arg("build")
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{said}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The built page at `rel` inside `public/`, or `None` when there is none.
#[cfg(test)]
fn page(site: &TempDir, rel: &str) -> Option<String> {
    fs::read_to_string(at(site.path(), &format!("public/{rel}"))).ok()
}

/// The page's main area, where the list is.
#[cfg(test)]
fn main_of(page: &str) -> &str {
    let (_, main) = page.split_once("<main").unwrap();
    main.split_once("</main>").unwrap().0
}

#[test]
fn the_archive_splits_every_post_into_pages_of_twenty_and_stays_out_of_search() {
    let site = site(21, "");
    build(&site);
    let first = page(&site, "archive/index.html").unwrap();
    assert_eq!(first.matches(r#"<article class="card">"#).count(), 20);
    assert!(first.contains("<h1>Archive</h1>"), "{first}");
    assert!(first.contains(r#"href="/blog/archive/page/2/""#), "{first}");
    assert!(
        first.contains("noindex"),
        "the home page lists the same posts"
    );
    let second = page(&site, "archive/page/2/index.html").unwrap();
    assert_eq!(second.matches(r#"<article class="card">"#).count(), 1);
    assert!(second.contains("<h1>Archive, page 2</h1>"), "{second}");
    assert!(second.contains("Post 1</a>"), "the oldest post is last");
}

#[test]
fn the_authors_page_lists_each_author_with_their_post_count() {
    let site = site(3, "");
    build(&site);
    let authors = page(&site, "authors/index.html").unwrap();
    insta::assert_snapshot!(main_of(&authors));
}

#[test]
fn a_site_can_turn_the_archive_and_the_authors_page_off() {
    let site = site(3, "\n[reader]\narchive = false\nauthors_page = false\n");
    build(&site);
    assert_eq!(page(&site, "archive/index.html"), None);
    assert_eq!(page(&site, "authors/index.html"), None);
    assert!(page(&site, "authors/asif/index.html").is_some());
}
