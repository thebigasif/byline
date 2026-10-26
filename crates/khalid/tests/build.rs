//! `khalid build`: the whole site rendered as HTML into an output folder
//! (RULES.md BL-CLI-1 to BL-CLI-3, BL-FS-2).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

#[cfg(test)]
fn khalid(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

#[cfg(test)]
fn path(folder: &Path) -> &str {
    folder.to_str().unwrap()
}

#[cfg(test)]
fn demo_site() -> PathBuf {
    at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    )
}

#[cfg(test)]
fn page(out: &Path, rel: &str) -> String {
    fs::read_to_string(at(out, rel)).unwrap_or_else(|error| panic!("{rel}: {error}"))
}

#[test]
fn build_renders_every_post_and_home_page_of_the_demo_site() {
    let out = tempfile::tempdir().unwrap();
    let demo = demo_site();
    let output = khalid(&["--site", path(&demo), "build", "--out", path(out.path())]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(
        text(&output.stdout).contains("20 pages"),
        "{}",
        text(&output.stdout)
    );

    let post = page(out.path(), "why-internal-links-matter/index.html");
    assert!(post.contains("<html lang=\"en\""), "{post}");
    assert!(post.contains("Why internal links matter</h1>"), "{post}");
    assert!(
        post.contains("<h2 id=\"readers-use-them-to-keep-going\""),
        "{post}"
    );
    assert!(post.contains("https://www.example.com/blog/why-internal-links-matter/"));

    let german = page(out.path(), "de/warum-interne-links-wichtig-sind/index.html");
    assert!(german.contains("<html lang=\"de\""), "{german}");

    let home = page(out.path(), "index.html");
    let newest = home.find("/blog/planning-a-topic-cluster/");
    let oldest = home.find("/blog/why-internal-links-matter/");
    assert!(newest.is_some() && newest < oldest, "newest first:\n{home}");
    assert!(page(out.path(), "de/index.html").contains("/blog/de/"));
    assert!(page(out.path(), "404.html").contains("<h1"));
    assert!(page(out.path(), "de/404.html").contains("lang=\"de\""));
}

#[test]
fn build_writes_the_author_and_plain_pages_every_link_points_to() {
    let out = tempfile::tempdir().unwrap();
    let demo = demo_site();
    let output = khalid(&["--site", path(&demo), "build", "--out", path(out.path())]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));

    let post = page(out.path(), "what-a-sitemap-does/index.html");
    assert!(post.contains("/blog/authors/lena-vogt/"), "{post}");
    let author = page(out.path(), "authors/lena-vogt/index.html");
    assert!(author.contains("Lena Vogt</h1>"), "{author}");
    assert!(author.contains("/blog/what-a-sitemap-does/"), "{author}");
    assert!(
        !author.contains("/blog/why-internal-links-matter/"),
        "{author}"
    );
    assert!(page(out.path(), "de/authors/asif/index.html").contains("lang=\"de\""));

    assert!(page(out.path(), "about/index.html").contains("About Field Notes</h1>"));
    assert!(page(out.path(), "de/ueber-uns/index.html").contains("Über Field Notes</h1>"));
}

#[test]
fn init_then_new_then_build_writes_the_post_to_public() {
    let temp = tempfile::tempdir().unwrap();
    let site = path(temp.path());
    let made = khalid(&[
        "init",
        site,
        "--address",
        "https://www.example.com",
        "--mount",
        "/blog/",
    ]);
    assert_eq!(made.status.code(), Some(0), "{}", text(&made.stderr));
    let new = khalid(&["--site", site, "new", "Why links matter"]);
    assert_eq!(new.status.code(), Some(0), "{}", text(&new.stderr));

    let output = khalid(&["--site", site, "build", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let public = at(temp.path(), "public");
    assert_eq!(json["out"], path(&public));
    assert_eq!(json["pages"], 3);
    let post = page(&public, "why-links-matter/index.html");
    assert!(post.contains("Why links matter</h1>"), "{post}");
}

#[test]
fn build_copies_a_folder_posts_image_beside_its_page() {
    let out = tempfile::tempdir().unwrap();
    let demo = demo_site();
    let output = khalid(&["--site", path(&demo), "build", "--out", path(out.path())]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let chart = "content/en/posts/2026-10-05-fixing-slow-pages/page-weight.png";
    assert_eq!(
        fs::read(at(out.path(), "fixing-slow-pages/page-weight.png")).unwrap(),
        fs::read(at(&demo, chart)).unwrap(),
    );
    let post = page(out.path(), "fixing-slow-pages/index.html");
    assert!(
        post.contains("src=\"/blog/fixing-slow-pages/page-weight.png\""),
        "{post}"
    );
}

#[test]
fn build_needs_a_site_folder() {
    let output = khalid(&["build"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("--site"));
}

#[test]
fn build_refuses_to_write_into_the_content_folder() {
    let temp = tempfile::tempdir().unwrap();
    let site = path(temp.path());
    khalid(&["init", site, "--address", "https://www.example.com"]);
    let content = at(temp.path(), "content/html");
    let output = khalid(&["--site", site, "build", "--out", path(&content), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["error"]["why"].as_str().unwrap().contains("content"));
    assert!(!content.exists());
}
