//! A `#` link on every H2 and H3 of a post, so a reader can share one
//! section's address. A plain link: it works with scripts off.

use khalid_render::Feature;

use crate::contents::long_post;
use crate::html_check::assert_valid_page;
use crate::post::{part, render, render_on, site_without};

#[test]
fn each_section_heading_links_to_itself() {
    let page = render("en", long_post());
    assert_valid_page(&page);
    insta::assert_snapshot!(part(&page, "<div class=\"post-body\">", "</div>"));
}

#[test]
fn the_link_is_named_in_the_page_language() {
    let page = render("de", long_post());
    assert!(page.contains(
        r##"<a class="heading-link" href="#by-hand" aria-label="Link zu diesem Abschnitt">#</a></h3>"##
    ));
}

#[test]
fn switched_off_headings_have_no_links() {
    let page = render_on(&site_without(Feature::HeadingLinks), long_post());
    assert_valid_page(&page);
    assert!(page.contains("<h2 id=\"what-it-does\">What it does</h2>"));
    assert!(!page.contains("heading-link"));
}
