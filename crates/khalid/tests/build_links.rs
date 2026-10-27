//! `khalid build` runs every link in a rendered body through the site's
//! link policy from `[seo.links]` (KH-SEO-10, audit G17).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// External links `nofollow` and in a new tab, Amazon an affiliate.
const LINKS: &str = "\n[seo.links]\nrel = [\"nofollow\"]\nopen = \"new-tab\"\naffiliate_domains = [\"amazon.com\"]\n";

/// A post with a plain external link, an affiliate link and a link inside
/// the blog.
const POST: &str = "---\ntitle: Books on links\nslug: books-on-links\ndate: 2026-10-01\nauthor: asif\n---\n\n\
Google's [link guide](https://developers.google.com/search/docs/crawling-indexing/links-crawlable) \
is free. [The book](https://www.amazon.com/dp/B0C1234567) is not. Start with \
[the home page](/blog/).\n";

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

/// A site at `/blog/` with `extra` added to its settings and [`POST`].
#[cfg(test)]
fn site(extra: &str) -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    let settings = format!(
        "name = \"Field Notes on SEO\"\naddress = \"https://www.example.com\"\nmount = \"/blog/\"\n\
         mode = \"static\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n{extra}"
    );
    fs::write(at(temp.path(), "site.toml"), settings).unwrap();
    let folder = at(temp.path(), "content/en/posts");
    fs::create_dir_all(&folder).unwrap();
    fs::write(at(&folder, "books-on-links.md"), POST).unwrap();
    temp
}

/// The links in the body of the built post, one per line.
#[cfg(test)]
fn body_links(site: &TempDir) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site.path())
        .arg("build")
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let page = fs::read_to_string(at(site.path(), "public/books-on-links/index.html")).unwrap();
    let (_, body) = page.split_once("Google's ").unwrap();
    let (body, _) = body.split_once("</p>").unwrap();
    body.split("<a ")
        .skip(1)
        .map(|link| format!("<a {}", link.split_once("</a>").unwrap().0))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_plain_an_affiliate_and_an_internal_link_get_the_policy() {
    insta::assert_snapshot!(body_links(&site(LINKS)));
}

#[test]
fn without_the_table_every_link_is_left_alone() {
    let links = body_links(&site(""));
    assert!(!links.contains("rel="), "{links}");
    assert!(!links.contains("target="), "{links}");
}
