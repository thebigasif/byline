//! `khalid build` writes `_redirects` from the `aliases` in post front
//! matter, one hop per line, for static hosts (KH-SEO-8).

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

/// A site with one post per `(slug, aliases)` pair, each alias on its own
/// line of the post's `aliases` list. Each post has an author and an
/// internal link, so the publish gate lets it through.
#[cfg(test)]
fn site(posts: &[(&str, &[&str])]) -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::write(at(temp.path(), "site.toml"), SITE_TOML).unwrap();
    let folder = at(temp.path(), "content/en/posts");
    fs::create_dir_all(&folder).unwrap();
    for (slug, aliases) in posts {
        let mut text =
            format!("---\ntitle: {slug}\nslug: {slug}\ndate: 2026-10-01\nauthor: asif\naliases:\n");
        for alias in *aliases {
            text.push_str("  - ");
            text.push_str(alias);
            text.push('\n');
        }
        text.push_str("---\n\nA post that moved from [the home page](/blog/).\n");
        fs::write(at(&folder, &format!("{slug}.md")), text).unwrap();
    }
    temp
}

#[cfg(test)]
fn build(site: &TempDir) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site.path())
        .arg("build")
        .output()
        .unwrap()
}

#[test]
fn aliases_become_one_line_each_with_the_post_address() {
    let site = site(&[
        (
            "why-links-matter",
            &["/blog/2019/05/links/", "/blog/links/"],
        ),
        ("title-tags", &["/blog/titles/"]),
    ]);
    let output = build(&site);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let redirects = fs::read_to_string(at(site.path(), "public/_redirects")).unwrap();
    insta::assert_snapshot!(redirects);
}

#[test]
fn an_alias_lands_where_routes_toml_puts_the_post() {
    let site = site(&[("why-links-matter", &["/blog/links/"])]);
    let routes = "[collections.posts]\npermalink = \"/{year}/{slug}/\"\n";
    fs::write(at(site.path(), "routes.toml"), routes).unwrap();
    let output = build(&site);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let redirects = fs::read_to_string(at(site.path(), "public/_redirects")).unwrap();
    assert_eq!(redirects, "/blog/links/ /blog/2026/why-links-matter/ 301\n");
    assert!(at(site.path(), "public/2026/why-links-matter/index.html").is_file());
}

#[test]
fn a_site_without_aliases_has_no_redirects_file() {
    let site = site(&[("why-links-matter", &[])]);
    assert_eq!(build(&site).status.code(), Some(0));
    assert!(!at(site.path(), "public/_redirects").exists());
}

#[test]
fn an_alias_outside_the_blog_stops_the_build() {
    let site = site(&[("why-links-matter", &["/old/links/"])]);
    let output = build(&site);
    assert_eq!(output.status.code(), Some(1));
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("/old/links/"), "{message}");
    assert!(message.contains("/blog/"), "{message}");
}

#[test]
fn two_posts_claiming_one_alias_stop_the_build() {
    let site = site(&[
        ("why-links-matter", &["/blog/links/"]),
        ("title-tags", &["/blog/links/"]),
    ]);
    let output = build(&site);
    assert_eq!(output.status.code(), Some(1));
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("/links/"), "{message}");
}
