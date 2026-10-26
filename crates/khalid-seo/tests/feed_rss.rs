//! RSS 2.0 feeds (task 1.13, KH-SEO-6), read back with the `rss` crate's
//! strict reader and its validator.

#![cfg(test)]

mod common;

use common::{at, demo_site, internal_links_post, internal_links_post_de, locale};
use khalid_seo::{Feed, FeedFormat, FeedScope, MetaTemplate};
use rss::Channel;
use rss::validation::Validate;

fn demo_feed() -> Feed {
    let mut site = demo_site();
    site.feeds.source_line =
        Some(MetaTemplate::parse("{title} first appeared on {site}.").unwrap());
    let mut post = internal_links_post();
    post.content_html = String::from("<p>Link from <em>old</em> posts to new ones.</p>");
    Feed::for_pages(
        &site,
        &[post],
        &FeedScope::language(locale("en")),
        at(2026, 10, 26, 12),
    )
    .unwrap()
}

#[test]
fn the_demo_rss_feed() {
    insta::assert_snapshot!(demo_feed().to_rss().unwrap());
}

#[test]
fn the_rss_feed_parses_and_validates() {
    let xml = demo_feed().to_rss().unwrap();
    let channel = Channel::read_from(xml.as_bytes()).unwrap();
    channel.validate().unwrap();
    assert_eq!(channel.items().len(), 1);
    let item = &channel.items()[0];
    assert_eq!(item.title(), Some("Why internal links matter"));
    assert_eq!(
        item.content(),
        Some(
            "<p>Link from <em>old</em> posts to new ones.</p><p><a href=\"https://www.example.com/blog/why-internal-links-matter/\">\
             Why internal links matter first appeared on Field Notes on SEO.</a></p>"
        )
    );
}

#[test]
fn an_empty_feed_still_validates() {
    let mut site = demo_site();
    site.tagline = String::new();
    let feed = Feed::for_pages(
        &site,
        &[internal_links_post_de()],
        &FeedScope::language(locale("en")),
        at(2026, 10, 26, 12),
    )
    .unwrap();
    let channel = Channel::read_from(feed.to_rss().unwrap().as_bytes()).unwrap();
    channel.validate().unwrap();
    assert_eq!(channel.description(), "Field Notes on SEO");
}

#[test]
fn each_format_has_its_own_address_next_to_the_feed_page() {
    let feed = demo_feed();
    assert_eq!(
        feed.url(FeedFormat::Rss).unwrap().as_str(),
        "https://www.example.com/blog/feed.xml"
    );
    assert_eq!(
        feed.url(FeedFormat::Atom).unwrap().as_str(),
        "https://www.example.com/blog/atom.xml"
    );
    assert_eq!(
        feed.url(FeedFormat::Json).unwrap().as_str(),
        "https://www.example.com/blog/feed.json"
    );
}
