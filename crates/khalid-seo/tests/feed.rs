//! The feed model shared by RSS, Atom and JSON Feed (task 1.13, KH-SEO-6).

#![cfg(test)]

mod common;

use common::{at, demo_site, internal_links_post, internal_links_post_de, locale, path};
use khalid_seo::{Feed, FeedContent, FeedScope, MetaTemplate, PageKind, PublishState, SeoPage};

fn post(slug: &str, title: &str, day: u32, category: &str) -> SeoPage {
    let mut page = SeoPage::new(
        PageKind::Post,
        locale("en"),
        path(&format!("/{slug}/")),
        title,
    );
    page.published = Some(at(2026, 10, day, 9));
    page.category = Some(category.to_owned());
    page.excerpt = format!("A short note on {title}.");
    page.content_html = format!("<p>All about {title}.</p>");
    page
}

fn demo_pages() -> Vec<SeoPage> {
    let mut draft = post("draft", "A draft", 25, "On-page SEO");
    draft.state = PublishState::Draft;
    let about = SeoPage::new(PageKind::Page, locale("en"), path("/about/"), "About");
    let mut links = internal_links_post();
    links.content_html = String::from("<p>Link from old posts to new ones.</p>");
    let mut german = internal_links_post_de();
    german.published = Some(at(2026, 9, 21, 8));
    vec![
        links,
        post("anchor-text", "Anchor text", 20, "On-page SEO"),
        post("crawl-budget", "Crawl budget", 22, "Technical SEO"),
        german,
        draft,
        about,
    ]
}

fn titles(feed: &Feed) -> Vec<&str> {
    feed.items.iter().map(|item| item.title.as_str()).collect()
}

#[test]
fn a_language_feed_holds_its_published_posts_newest_first() {
    let site = demo_site();
    let feed = Feed::for_pages(
        &site,
        &demo_pages(),
        &FeedScope::language(locale("en")),
        at(2026, 10, 26, 12),
    )
    .unwrap();
    assert_eq!(
        titles(&feed),
        ["Crawl budget", "Anchor text", "Why internal links matter"]
    );
    assert_eq!(feed.title, "Field Notes on SEO");
    assert_eq!(feed.home.as_str(), "https://www.example.com/blog/");
    assert_eq!(feed.updated, at(2026, 10, 22, 9));
}

#[test]
fn a_category_feed_holds_only_that_category() {
    let site = demo_site();
    let scope = FeedScope::category(locale("en"), "On-page SEO", path("/category/on-page-seo/"));
    let feed = Feed::for_pages(&site, &demo_pages(), &scope, at(2026, 10, 26, 12)).unwrap();
    assert_eq!(titles(&feed), ["Anchor text", "Why internal links matter"]);
    assert_eq!(feed.title, "On-page SEO | Field Notes on SEO");
    assert_eq!(
        feed.home.as_str(),
        "https://www.example.com/blog/category/on-page-seo/"
    );
}

#[test]
fn the_german_feed_holds_the_german_post() {
    let site = demo_site();
    let feed = Feed::for_pages(
        &site,
        &demo_pages(),
        &FeedScope::language(locale("de")),
        at(2026, 10, 26, 12),
    )
    .unwrap();
    assert_eq!(titles(&feed), ["Warum interne Links wichtig sind"]);
    assert_eq!(feed.home.as_str(), "https://www.example.com/blog/de/");
}

#[test]
fn full_content_carries_the_text_before_and_after_and_the_source_line() {
    let mut site = demo_site();
    site.feeds.before_item = String::from("New on Field Notes:");
    site.feeds.after_item = String::from("Thanks for reading & sharing.");
    site.feeds.source_line =
        Some(MetaTemplate::parse("{title} first appeared on {site}.").unwrap());
    let feed = Feed::for_pages(
        &site,
        &demo_pages(),
        &FeedScope::language(locale("en")),
        at(2026, 10, 26, 12),
    )
    .unwrap();
    insta::assert_snapshot!(feed.items[2].content_html);
}

#[test]
fn excerpt_mode_carries_the_excerpt_in_place_of_the_body() {
    let mut site = demo_site();
    site.feeds.content = FeedContent::Excerpt;
    let feed = Feed::for_pages(
        &site,
        &demo_pages(),
        &FeedScope::language(locale("en")),
        at(2026, 10, 26, 12),
    )
    .unwrap();
    assert_eq!(
        feed.items[0].content_html,
        "<p>A short note on Crawl budget.</p>"
    );
    assert_eq!(feed.items[0].summary, "A short note on Crawl budget.");
}

#[test]
fn the_feed_is_cut_at_the_limit_and_an_empty_feed_is_dated_now() {
    let mut site = demo_site();
    site.feeds.limit = 2;
    let now = at(2026, 10, 26, 12);
    let feed = Feed::for_pages(
        &site,
        &demo_pages(),
        &FeedScope::language(locale("en")),
        now,
    )
    .unwrap();
    assert_eq!(feed.items.len(), 2);
    let empty = Feed::for_pages(&site, &[], &FeedScope::language(locale("en")), now).unwrap();
    assert_eq!(empty.updated, now);
}
