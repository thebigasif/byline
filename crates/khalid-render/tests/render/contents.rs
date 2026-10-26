//! The automatic table of contents: a `details` block above the body on
//! phones and a sticky side column on wide screens, for posts with three or
//! more sections. Plain links, so it works with scripts off.

use khalid_render::{BodyHtml, Contents, ContentsHeading, Feature, HeadingLevel, RenderPost};

use crate::html_check::assert_valid_page;
use crate::post::{part, render, render_on, site_without, sitemap_post};

fn heading(level: HeadingLevel, id: &str, text: &str) -> ContentsHeading {
    ContentsHeading {
        level,
        id: id.to_owned(),
        text: text.to_owned(),
    }
}

/// The demo sitemap post with three sections and two parts.
pub(crate) fn long_post() -> RenderPost {
    let mut post = sitemap_post();
    post.body = BodyHtml::from_sanitised(
        "<p>A sitemap lists the pages you want found.</p>\n\
         <h2 id=\"what-it-does\">What it does</h2>\n<p>It helps new pages get found.</p>\n\
         <h3 id=\"new-pages\">New pages</h3>\n<p>They show up sooner.</p>\n\
         <h2 id=\"what-it-does-not\">What it does not</h2>\n<p>It does not lift rankings.</p>\n\
         <h2 id=\"how-to-make-one\">How to make one</h2>\n<p>Khalid writes it for you.</p>\n\
         <h3 id=\"by-hand\">By hand</h3>\n<p>List each address.</p>"
            .to_owned(),
    );
    post.contents = Contents::of(vec![
        heading(HeadingLevel::Two, "what-it-does", "What it does"),
        heading(HeadingLevel::Three, "new-pages", "New pages"),
        heading(HeadingLevel::Two, "what-it-does-not", "What it does not"),
        heading(HeadingLevel::Two, "how-to-make-one", "How to make one"),
        heading(HeadingLevel::Three, "by-hand", "By hand"),
    ]);
    post
}

#[test]
fn a_long_post_lists_its_sections_above_the_body_and_beside_it() {
    let page = render("en", long_post());
    assert_valid_page(&page);
    assert!(page.contains(r#"<article class="post post-with-toc">"#));
    insta::assert_snapshot!(part(&page, "<details", "</nav>\n<div"));
}

#[test]
fn the_contents_read_in_the_page_language() {
    let page = render("de", long_post());
    assert!(page.contains("<summary>Inhalt</summary>"));
    assert!(page.contains(r#"<nav class="toc-side" aria-label="Inhalt">"#));
}

#[test]
fn a_short_post_has_no_contents() {
    let page = render("en", sitemap_post());
    assert!(page.contains(r#"<article class="post">"#));
    assert!(!page.contains("toc-"));
}

#[test]
fn switched_off_a_long_post_has_no_contents() {
    let page = render_on(&site_without(Feature::TableOfContents), long_post());
    assert_valid_page(&page);
    assert!(!page.contains("toc-"));
    assert!(!page.contains("post-with-toc"));
}
