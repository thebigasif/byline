//! A page read once into what the checks need.

use khalid_audit::{Block, Page};
use khalid_core::AbsoluteUrl;

const HEAD: &str = r#"<title>Hello</title>
<meta name="description" content="About it">
<meta name="robots" content="noindex, follow">
<link rel="canonical" href="https://www.example.com/blog/">
<link rel="alternate" hreflang="en" href="https://www.example.com/blog/en/">
<meta property="og:title" content="Hello">
<link rel="stylesheet" href="/blog/assets/site.css">
<script type="application/ld+json">{"@type":"WebSite"}</script>
<script src="app.js"></script>"#;

const BODY: &str = r#"<h1>Title one</h1><h1>Title  two</h1>
<p>Some words to count here.</p>
<a href="/blog/next/#part" rel="nofollow">Next post</a>
<a href="mailto:a@example.com">Mail</a>
<a href="/blog/"><img src="logo.png" alt="Home"></a>
<img src="/blog/media/a.png" alt="">
<img src="/blog/media/b.png">
<style>p { color: red }</style>
<svg><title>Not the page title</title></svg>"#;

#[cfg(test)]
fn read(head: &str, body: &str) -> Page {
    let html =
        format!("<!doctype html><html lang=\"de\"><head>{head}</head><body>{body}</body></html>");
    let url = AbsoluteUrl::parse("https://www.example.com/blog/post/").unwrap();
    Page::read(&url, html.as_bytes())
}

#[test]
fn reads_the_head() {
    let page = read(HEAD, BODY);
    assert_eq!(page.titles(), ["Hello"]);
    assert_eq!(page.descriptions(), ["About it"]);
    assert_eq!(page.language(), Some("de"));
    assert!(page.robots().noindex());
    assert!(!page.robots().nofollow());
    assert_eq!(
        page.canonical().map(AbsoluteUrl::as_str),
        Some("https://www.example.com/blog/")
    );
    let alternate = page.alternates().first().unwrap();
    assert_eq!(alternate.language(), "en");
    assert_eq!(alternate.to().as_str(), "https://www.example.com/blog/en/");
    assert_eq!(
        page.social().get("og:title").map(String::as_str),
        Some("Hello")
    );
    assert_eq!(page.structured_data(), [r#"{"@type":"WebSite"}"#]);
}

#[test]
fn reads_the_body() {
    let page = read(HEAD, BODY);
    assert_eq!(page.headings(), ["Title one", "Title two"]);
    let links: Vec<(&str, &str, bool)> = page
        .links()
        .iter()
        .map(|link| (link.to().as_str(), link.text(), link.nofollow()))
        .collect();
    assert_eq!(
        links,
        [
            ("https://www.example.com/blog/next/", "Next post", true),
            ("https://www.example.com/blog/", "Home", false),
        ]
    );
    let alts: Vec<Option<&str>> = page.images().iter().map(|image| image.alt()).collect();
    assert_eq!(alts, [Some("Home"), Some(""), None]);
    let resources: Vec<&str> = page.resources().iter().map(AbsoluteUrl::as_str).collect();
    assert_eq!(
        resources,
        [
            "https://www.example.com/blog/assets/site.css",
            "https://www.example.com/blog/post/app.js",
            "https://www.example.com/blog/post/logo.png",
            "https://www.example.com/blog/media/a.png",
            "https://www.example.com/blog/media/b.png",
        ]
    );
    // Headings, paragraph, link text and the drawing's title count, 16
    // words. The style does not.
    assert_eq!(page.words(), 16);
    assert!(page.text_bytes() > 0);
}

#[test]
fn a_page_with_nothing_in_it_reads_as_empty() {
    let page = read("", "");
    assert_eq!(page.titles(), Vec::<String>::new());
    assert_eq!(page.headings(), Vec::<String>::new());
    assert!(page.canonical().is_none());
    assert_eq!(page.words(), 0);
    assert_eq!(page.outline(), []);
    assert_eq!(page.folded(), Vec::<String>::new());
    assert_eq!(page.dates(), Vec::<String>::new());
}

/// A post body with a header and a footer around it.
const ARTICLE: &str = r##"<header><nav><p>Menu text</p></nav><h2>Site name?</h2></header>
<main><h1>Rye bread</h1>
<p>Published <time datetime="2026-03-01">1 March</time>.</p>
<h2 id="long">How long does  rye take? <a class="heading-link" href="#long">#</a></h2>
<p>About <a href="https://example.org/">three hours</a>.</p>
<p>Then it   rests.</p>
<details><summary>Can I freeze it?</summary><p>Yes.</p></details>
<details open><summary>Open one</summary><p>Shown.</p></details>
<h3>Flour</h3></main>
<footer><p>Footer text</p></footer>"##;

#[test]
fn reads_the_outline_an_answer_engine_lifts() {
    let page = read("", ARTICLE);
    let outline: Vec<(&str, &str, bool)> = page
        .outline()
        .iter()
        .map(|block| match block {
            Block::Heading(text) => ("heading", text.as_str(), false),
            Block::Paragraph(paragraph) => ("paragraph", paragraph.text(), paragraph.linked()),
            _ => ("other", "", false),
        })
        .collect();
    assert_eq!(
        outline,
        [
            ("paragraph", "Published 1 March.", false),
            ("heading", "How long does rye take?", false),
            ("paragraph", "About three hours.", true),
            ("paragraph", "Then it rests.", false),
            ("paragraph", "Yes.", false),
            ("paragraph", "Shown.", false),
            ("heading", "Flour", false),
        ],
        "the header, the menu and the footer are not the page's text"
    );
    assert_eq!(page.folded(), ["Can I freeze it?"]);
    assert_eq!(page.dates(), ["2026-03-01"]);
}

#[test]
fn in_an_article_comments_menus_and_forms_are_not_the_page_text() {
    let body = r##"<article><header><h1>Rye</h1><p>Published <time datetime="2026-03-01">then</time></p></header>
<details><summary>Contents</summary><nav><ul><li><a href="#a">A</a></li></ul></nav></details>
<div><h2>Why rye?</h2><p>It keeps.</p></div>
<footer><h2>Tags</h2></footer>
<section><article><p>A comment with 3 numbers.</p></article>
<details><summary>Reply</summary><form><p><label>Name</label></p></form></details></section>
</article>"##;
    let page = read("", body);
    assert_eq!(
        page.outline(),
        [
            Block::Heading("Why rye?".to_owned()),
            read("", "<p>It keeps.</p>")
                .outline()
                .first()
                .cloned()
                .unwrap(),
        ]
    );
    assert_eq!(page.folded(), Vec::<String>::new());
    assert_eq!(page.dates(), ["2026-03-01"]);
}
