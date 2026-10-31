//! `khalid build` writes each page's SEO head from `khalid-seo`: its tags,
//! hreflang between language versions, and its JSON-LD graph, on the home,
//! post, page, author, glossary, search and not-found pages (KH-SEO-2,
//! KH-SEO-3, KH-SEO-9).

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

/// The demo site, built once per test into a new folder.
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

/// The `<head>` of the page at `rel`.
#[cfg(test)]
fn head(out: &TempDir, rel: &str) -> String {
    let page = fs::read_to_string(at(out.path(), rel)).unwrap();
    let start = page.find("<head>").unwrap();
    let end = page.find("</head>").unwrap();
    page.get(start..end).unwrap().to_owned()
}

#[test]
fn the_home_page_head() {
    insta::assert_snapshot!(head(&built(), "index.html"));
}

#[test]
fn a_post_head() {
    insta::assert_snapshot!(head(&built(), "why-internal-links-matter/index.html"));
}

#[test]
fn an_author_page_head() {
    insta::assert_snapshot!(head(&built(), "authors/asif/index.html"));
}

#[test]
fn a_page_head() {
    insta::assert_snapshot!(head(&built(), "about/index.html"));
}

#[test]
fn a_glossary_term_head() {
    insta::assert_snapshot!(head(&built(), "glossary/topical-authority/index.html"));
}

#[test]
fn the_search_page_head() {
    insta::assert_snapshot!(head(&built(), "search/index.html"));
}

#[test]
fn the_not_found_page_head() {
    insta::assert_snapshot!(head(&built(), "404.html"));
}

#[test]
fn a_post_and_its_translation_name_each_other_and_x_default() {
    let out = built();
    let english = "https://www.example.com/blog/why-internal-links-matter/";
    let german = "https://www.example.com/blog/de/warum-interne-links-wichtig-sind/";
    let tags = [
        format!(r#"<link rel="alternate" hreflang="en" href="{english}">"#),
        format!(r#"<link rel="alternate" hreflang="de" href="{german}">"#),
        format!(r#"<link rel="alternate" hreflang="x-default" href="{english}">"#),
    ];
    for rel in [
        "why-internal-links-matter/index.html",
        "de/warum-interne-links-wichtig-sind/index.html",
    ] {
        let head = head(&out, rel);
        for tag in &tags {
            assert!(head.contains(tag.as_str()), "{rel} lacks {tag}:\n{head}");
        }
    }
}

#[test]
fn the_home_pages_of_both_languages_name_each_other() {
    let out = built();
    for rel in ["index.html", "de/index.html"] {
        let head = head(&out, rel);
        assert!(
            head.contains(
                r#"<link rel="alternate" hreflang="de" href="https://www.example.com/blog/de/">"#
            ),
            "{rel}:\n{head}"
        );
    }
}

#[test]
fn each_head_has_one_title_and_one_schema_graph() {
    let out = built();
    let head = head(&out, "why-internal-links-matter/index.html");
    assert_eq!(head.matches("<title>").count(), 1, "{head}");
    assert_eq!(
        head.matches(r#"<script type="application/ld+json">"#)
            .count(),
        1,
        "{head}"
    );
    assert!(head.contains(r#""@type":"BlogPosting""#), "{head}");
    assert!(head.contains(r#"<meta name="robots" content="index, follow"#));
}

#[test]
fn the_not_found_page_is_kept_out_of_search() {
    let out = built();
    let head = head(&out, "404.html");
    assert!(
        head.contains(r#"<meta name="robots" content="noindex, follow">"#),
        "{head}"
    );
    assert!(!head.contains("hreflang"), "{head}");
}
