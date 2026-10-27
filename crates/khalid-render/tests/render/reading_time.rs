//! Reading time beside a post's date: "8 min read", as a machine-readable
//! duration, in the page's language, and gone when switched off.

use khalid_core::Locale;
use khalid_render::{BodyHtml, Feature, ReadingTime};

use crate::html_check::assert_valid_page;
use crate::post::{part, render, render_on, site_without, sitemap_post};

/// The demo post with a body of `words` words.
fn post_of(words: usize) -> khalid_render::RenderPost {
    let mut post = sitemap_post();
    post.body = BodyHtml::from_sanitised(format!("<p>{}</p>", vec!["word"; words].join(" ")));
    post.reading_time = ReadingTime::of(&post.body, &Locale::try_new("en").unwrap());
    post
}

#[test]
fn a_post_shows_its_reading_time_beside_its_date() {
    let page = render("en", post_of(1840));
    assert_valid_page(&page);
    insta::assert_snapshot!(part(&page, "<p>Published", "</p>\n<p>Filed"));
}

#[test]
fn the_reading_time_reads_in_the_page_language() {
    let page = render("de", post_of(1840));
    assert!(page.contains(r#"<time datetime="PT8M">8 Min. Lesezeit</time>"#));
}

#[test]
fn a_short_post_reads_in_a_minute() {
    let page = render("en", sitemap_post());
    assert!(page.contains(r#"<time datetime="PT1M">1 min read</time>"#));
}

#[test]
fn switched_off_there_is_no_reading_time() {
    let page = render_on(&site_without(Feature::ReadingTime), post_of(1840));
    assert_valid_page(&page);
    assert!(!page.contains("min read"));
    assert!(!page.contains("reading-time"));
}
