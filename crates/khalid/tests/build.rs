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
fn build_writes_the_stylesheet_every_page_links_with_its_compressed_copies() {
    let out = tempfile::tempdir().unwrap();
    let demo = demo_site();
    let output = khalid(&["--site", path(&demo), "build", "--out", path(out.path())]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));

    let home = page(out.path(), "index.html");
    let (_, rest) = home.split_once(r#"<link rel="stylesheet" href=""#).unwrap();
    let (href, _) = rest.split_once('"').unwrap();
    let file = href.strip_prefix("/blog/").unwrap();
    let hash = file
        .strip_prefix("assets/site.")
        .and_then(|r| r.strip_suffix(".css"));
    assert_eq!(hash.map(str::len), Some(8), "{href}");
    assert!(
        page(out.path(), "de/index.html").contains(href),
        "one stylesheet"
    );
    let css = page(out.path(), file);
    assert!(css.contains("--color-background: #ffffff;"), "{css}");
    for copy in [".br", ".gz"] {
        let size = fs::metadata(at(out.path(), &format!("{file}{copy}")))
            .unwrap()
            .len();
        assert!(size > 0 && size < 20 * 1024, "{copy}: {size} bytes");
    }
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

    let public = at(temp.path(), "public");
    let refused = khalid(&["--site", site, "build"]);
    assert_eq!(
        refused.status.code(),
        Some(1),
        "a new post has no author yet"
    );
    assert!(text(&refused.stdout).contains("This post has no author."));
    assert!(!public.exists(), "a refused build writes nothing");

    let posts = at(temp.path(), "content/en/posts");
    let file = fs::read_dir(&posts)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let written = fs::read_to_string(&file).unwrap();
    let mut finished = written.replacen("---\n", "---\nauthor: asif\n", 1);
    finished.push_str(
        "Links help readers find [the home page](/blog/) and [a guide](https://example.org/).\n",
    );
    fs::write(&file, finished).unwrap();

    let output = khalid(&["--site", site, "build", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["out"], path(&public));
    assert_eq!(json["pages"], 3);
    let post = page(&public, "why-links-matter/index.html");
    assert!(post.contains("Why links matter</h1>"), "{post}");
}

/// A post that passes every block.
const READY: &str = "---
title: Why internal links matter
date: 2026-09-14
author: asif
description: Internal links help readers and search engines find the rest of your site.
---

An internal link points to [how to pick link text](/blog/choosing-link-text/).
";

/// A dated post with no author and no internal link: two blocks.
const BLOCKED: &str = "---
title: Choosing link text
date: 2026-09-21
---

Good link text names the page it opens.
";

/// A site folder with the demo settings and `posts` as `(file name, text)`.
#[cfg(test)]
fn site(posts: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::copy(at(&demo_site(), "site.toml"), at(dir.path(), "site.toml")).unwrap();
    let folder = at(dir.path(), "content/en/posts");
    fs::create_dir_all(&folder).unwrap();
    for (name, text) in posts {
        fs::write(at(&folder, name), text).unwrap();
    }
    dir
}

#[test]
fn build_refuses_a_post_the_gate_blocks_and_explains_every_finding() {
    let site = site(&[
        ("2026-09-14-why-internal-links-matter.md", READY),
        ("2026-09-21-choosing-link-text.md", BLOCKED),
    ]);
    let output = khalid(&["--site", path(site.path()), "build"]);
    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stdout);
    assert!(
        said.contains("en/choosing-link-text: blocks publishing"),
        "{said}"
    );
    assert!(said.contains("This post has no author."), "{said}");
    assert!(said.contains("Rule no-internal-link"), "{said}");
    assert!(!said.contains("en/why-internal-links-matter"), "{said}");
    let why = text(&output.stderr);
    assert!(why.contains("1 post is not ready"), "{why}");
    assert_eq!(why.lines().count(), 3, "what, why, fix:\n{why}");
    assert!(!at(site.path(), "public").exists(), "nothing is written");
}

#[test]
fn build_json_lists_the_refused_posts_and_exits_1() {
    let site = site(&[("2026-09-21-choosing-link-text.md", BLOCKED)]);
    let output = khalid(&["--site", path(site.path()), "build", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout).lines().count(), 1, "one JSON value");
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["error"]["fix"].as_str().unwrap().contains("date:"));
    let refused = json["refused"].as_array().unwrap();
    assert_eq!(refused.len(), 1, "{json}");
    assert_eq!(refused[0]["blocks"], true);
    let rules: Vec<_> = refused[0]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["severity"] == "block")
        .map(|f| f["rule"].as_str().unwrap())
        .collect();
    assert_eq!(rules, ["no-author", "no-internal-link"]);
}

#[test]
fn build_reads_lint_toml_and_stops_on_a_mistake_in_it() {
    let site = site(&[("2026-09-21-choosing-link-text.md", BLOCKED)]);
    let settings = at(site.path(), "lint.toml");
    fs::write(&settings, "[rules]\nno-authors = \"off\"\n").unwrap();
    let output = khalid(&["--site", path(site.path()), "build"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("line 2"));

    let opened = "[rules]\nno-author = \"warn\"\nno-internal-link = \"off\"\n";
    fs::write(&settings, opened).unwrap();
    let output = khalid(&["--site", path(site.path()), "build"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
}

#[test]
fn build_leaves_drafts_to_the_writer() {
    let draft = BLOCKED.replace("date: 2026-09-21\n", "");
    let site = site(&[
        ("2026-09-14-why-internal-links-matter.md", READY),
        ("choosing-link-text.md", &draft),
    ]);
    let output = khalid(&["--site", path(site.path()), "build", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    let public = at(site.path(), "public");
    assert!(at(&public, "why-internal-links-matter/index.html").exists());
    assert!(!at(&public, "choosing-link-text").exists());
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
}

/// Every `.html` file under `folder`.
#[cfg(test)]
fn html_files(folder: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            html_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "html") {
            found.push(path);
        }
    }
}

#[test]
fn every_image_on_every_built_page_has_its_five_attributes() {
    let out = tempfile::tempdir().unwrap();
    let demo = demo_site();
    let output = khalid(&["--site", path(&demo), "build", "--out", path(out.path())]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let mut pages = Vec::new();
    html_files(out.path(), &mut pages);
    let mut images = 0;
    for page in pages {
        let html = fs::read_to_string(&page).unwrap();
        for tag in html.split("<img ").skip(1) {
            let tag = tag.split_once('>').unwrap().0;
            for attribute in ["width", "height", "srcset", "sizes", "alt"] {
                assert!(
                    tag.contains(&format!(" {attribute}=\"")) || tag.starts_with(attribute),
                    "{}: <img {tag}> has no {attribute}",
                    page.display()
                );
            }
            images += 1;
        }
    }
    assert!(images > 0, "the demo site shows at least one image");
    let post = page(out.path(), "fixing-slow-pages/index.html");
    let srcset = post.split("srcset=\"/blog/").nth(1).unwrap();
    let webp = srcset.split(' ').next().unwrap();
    assert!(
        webp.starts_with("media/") && webp.contains(".webp"),
        "{webp}"
    );
    assert!(at(out.path(), webp).is_file(), "{webp} is written");
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
