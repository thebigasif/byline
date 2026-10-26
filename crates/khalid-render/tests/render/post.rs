//! The post page: heading, byline, dates in the page's language, category,
//! tags and the sanitised body.

use chrono::NaiveDate;
use khalid_render::{BodyHtml, Link, PageBody, PageKind, RenderPost, Renderer};

use crate::common::{demo_site, meta};
use crate::html_check::assert_valid_page;

fn link(name: &str, href: &str) -> Link {
    Link {
        name: name.to_owned(),
        href: href.to_owned(),
    }
}

/// "What a sitemap does" from the demo site, with its body already rendered.
fn sitemap_post() -> RenderPost {
    RenderPost {
        title: "What a sitemap does, and what it does not".to_owned(),
        date: NaiveDate::from_ymd_opt(2026, 9, 28).unwrap(),
        updated: None,
        authors: vec![link("Lena Vogt", "/blog/authors/lena-vogt/")],
        category: Some(link("Technical SEO", "/blog/category/technical-seo/")),
        tags: vec![link("Sitemaps", "/blog/tag/sitemaps/")],
        body: BodyHtml::from_sanitised(
            "<p>A sitemap is a file that lists the addresses of the pages on your site.</p>\n\
             <h2>What it does</h2>\n<p>It helps new pages get found sooner.</p>"
                .to_owned(),
        ),
    }
}

fn render(tag: &str, post: RenderPost) -> String {
    let meta = meta(&post.title, "/blog/what-a-sitemap-does/", PageKind::Article);
    Renderer::new()
        .unwrap()
        .render(&demo_site(tag), &meta, &PageBody::Post(Box::new(post)))
        .unwrap()
}

#[test]
fn post_page_in_english() {
    let page = render("en", sitemap_post());
    assert_valid_page(&page);
    insta::assert_snapshot!(page);
}

#[test]
fn post_page_in_german_writes_the_date_the_german_way() {
    let page = render("de", sitemap_post());
    assert_valid_page(&page);
    assert!(page.contains(r#"<time datetime="2026-09-28">28. September 2026</time>"#));
    assert!(page.contains("<address>Von <a rel=\"author\""));
}

#[test]
fn an_update_shows_its_own_date() {
    let mut post = sitemap_post();
    post.updated = NaiveDate::from_ymd_opt(2026, 10, 3);
    let page = render("en", post);
    assert!(page.contains(r#"Updated <time datetime="2026-10-03">3 October 2026</time>"#));
}

#[test]
fn the_body_is_printed_as_sanitised_and_the_title_is_escaped() {
    let mut post = sitemap_post();
    post.title = "Sitemaps <b>and</b> you".to_owned();
    let page = render("en", post);
    assert!(page.contains("<h2>What it does</h2>"));
    assert!(page.contains("<h1>Sitemaps &lt;b&gt;and&lt;/b&gt; you</h1>"));
}

#[test]
fn several_authors_are_listed_in_order() {
    let mut post = sitemap_post();
    post.authors.push(link("Asif", "/blog/authors/asif/"));
    let page = render("en", post);
    assert!(page.contains(
        r#"By <a rel="author" href="/blog/authors/lena-vogt/">Lena Vogt</a>, <a rel="author" href="/blog/authors/asif/">Asif</a>"#
    ));
}

#[test]
fn a_post_without_tags_or_category_has_no_empty_sections() {
    let mut post = sitemap_post();
    post.tags.clear();
    post.category = None;
    let page = render("en", post);
    assert_valid_page(&page);
    assert!(!page.contains("post-footer"));
    assert!(!page.contains("Filed under"));
}
