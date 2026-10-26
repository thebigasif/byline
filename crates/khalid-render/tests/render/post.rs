//! The post page: heading, byline, dates in the page's language, category,
//! tags and the sanitised body.

use chrono::NaiveDate;
use khalid_render::{
    BodyHtml, Feature, Features, Link, PageBody, PageKind, Person, ReadingTime, RenderPost,
    RenderSite, Renderer,
};

use crate::common::{demo_site, locale, meta};
use crate::html_check::assert_valid_page;

pub(crate) fn link(name: &str, href: &str) -> Link {
    Link {
        name: name.to_owned(),
        href: href.to_owned(),
    }
}

/// An author with a role and nothing else.
pub(crate) fn person(name: &str, slug: &str, role: Option<&str>) -> Person {
    Person {
        name: name.to_owned(),
        href: format!("/blog/authors/{slug}/"),
        role: role.map(str::to_owned),
        image: None,
        bio: None,
        profiles: Vec::new(),
    }
}

/// "What a sitemap does" from the demo site, with its body already rendered.
pub(crate) fn sitemap_post() -> RenderPost {
    let body = BodyHtml::from_sanitised(
        "<p>A sitemap is a file that lists the addresses of the pages on your site.</p>\n\
         <h2>What it does</h2>\n<p>It helps new pages get found sooner.</p>"
            .to_owned(),
    );
    RenderPost {
        title: "What a sitemap does, and what it does not".to_owned(),
        date: NaiveDate::from_ymd_opt(2026, 9, 28).unwrap(),
        updated: None,
        reading_time: ReadingTime::of(&body, &locale("en")),
        authors: vec![person(
            "Lena Vogt",
            "lena-vogt",
            Some("Technical SEO editor"),
        )],
        reviewers: Vec::new(),
        category: Some(link("Technical SEO", "/blog/category/technical-seo/")),
        tags: vec![link("Sitemaps", "/blog/tag/sitemaps/")],
        image: None,
        contents: None,
        body,
    }
}

pub(crate) fn render(tag: &str, post: RenderPost) -> String {
    render_on(&demo_site(tag), post)
}

pub(crate) fn render_on(site: &RenderSite, post: RenderPost) -> String {
    let meta = meta(&post.title, "/blog/what-a-sitemap-does/", PageKind::Article);
    Renderer::new()
        .unwrap()
        .render(site, &meta, &PageBody::Post(Box::new(post)))
        .unwrap()
}

/// The part of `page` from the first `from` to the first `to` after it,
/// both included, for the snapshot of one feature.
pub(crate) fn part(page: &str, from: &str, to: &str) -> String {
    let (_, rest) = page.split_once(from).unwrap();
    let (inside, _) = rest.split_once(to).unwrap();
    format!("{from}{inside}{to}")
}

/// The English demo site with `feature` switched off.
pub(crate) fn site_without(feature: Feature) -> RenderSite {
    let mut site = demo_site("en");
    site.features = Features::all().without(feature);
    site
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
    assert!(page.contains(r#"<ul class="people" role="list" aria-label="Geschrieben von">"#));
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
fn a_post_without_tags_or_category_has_no_empty_sections() {
    let mut post = sitemap_post();
    post.tags.clear();
    post.category = None;
    let page = render("en", post);
    assert_valid_page(&page);
    assert!(!page.contains("post-footer"));
    assert!(!page.contains("Filed under"));
}
