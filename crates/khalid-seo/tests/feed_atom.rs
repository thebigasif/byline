//! Atom 1.0 feeds (task 1.13, KH-SEO-6), read back with
//! `atom_syndication` and checked against RFC 4287's required elements.

#![cfg(test)]

mod common;

use common::{at, demo_site, internal_links_post, internal_links_post_de, locale};
use khalid_seo::{Feed, FeedScope};

fn demo_feed() -> Feed {
    let mut post = internal_links_post();
    post.content_html = String::from("<p>Link from <em>old</em> posts to new ones.</p>");
    let mut german = internal_links_post_de();
    german.published = Some(at(2026, 9, 21, 8));
    german.language = locale("en");
    german.author = None;
    Feed::for_pages(
        &demo_site(),
        &[post, german],
        &FeedScope::language(locale("en")),
        at(2026, 10, 26, 12),
    )
    .unwrap()
}

/// RFC 4287 section 4.1: a feed and each entry have an id, a title and
/// an updated time, and every entry has an author, its own or the feed's.
fn check_required(feed: &atom_syndication::Feed) -> Result<(), String> {
    if feed.id().is_empty() || feed.title().is_empty() {
        return Err("the feed has no id or title".to_owned());
    }
    for entry in feed.entries() {
        if entry.id().is_empty() || entry.title().is_empty() {
            return Err(format!("an entry has no id or title: {entry:?}"));
        }
        if entry.authors().is_empty() && feed.authors().is_empty() {
            return Err(format!("an entry has no author: {}", entry.id()));
        }
        if entry.links().iter().all(|link| link.rel() != "alternate") {
            return Err(format!("an entry has no alternate link: {}", entry.id()));
        }
    }
    Ok(())
}

#[test]
fn the_demo_atom_feed() {
    insta::assert_snapshot!(demo_feed().to_atom().unwrap());
}

#[test]
fn the_atom_feed_parses_and_has_every_required_element() {
    let xml = demo_feed().to_atom().unwrap();
    let parsed = atom_syndication::Feed::read_from(xml.as_bytes()).unwrap();
    check_required(&parsed).unwrap();
    assert_eq!(parsed.entries().len(), 2);
    let content = parsed.entries()[1]
        .content()
        .and_then(|c| c.value())
        .unwrap();
    assert!(
        content.starts_with("<p>Link from <em>old</em>"),
        "{content}"
    );
    assert_eq!(parsed.lang(), Some("en"));
}

#[test]
fn an_entry_without_an_author_takes_the_feed_author() {
    let xml = demo_feed().to_atom().unwrap();
    let parsed = atom_syndication::Feed::read_from(xml.as_bytes()).unwrap();
    assert_eq!(parsed.authors()[0].name(), "Field Notes on SEO");
    assert_eq!(parsed.entries()[0].authors(), []);
    assert_eq!(parsed.entries()[1].authors()[0].name(), "Asif");
}

#[test]
fn the_required_element_check_refuses_an_entry_without_an_author() {
    let xml = "<?xml version=\"1.0\"?><feed xmlns=\"http://www.w3.org/2005/Atom\">\
               <id>https://example.com/</id><title>T</title><updated>2026-10-02T16:00:00Z</updated>\
               <entry><id>https://example.com/a/</id><title>A</title><updated>2026-10-02T16:00:00Z</updated>\
               <link rel=\"alternate\" href=\"https://example.com/a/\"/></entry></feed>";
    let parsed = atom_syndication::Feed::read_from(xml.as_bytes()).unwrap();
    assert!(check_required(&parsed).is_err());
}
