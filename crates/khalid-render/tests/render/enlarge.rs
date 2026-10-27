//! Click to enlarge: a body image drawn smaller than its original links to
//! its largest version. A plain link, so it works with scripts off.

use std::collections::BTreeMap;

use khalid_render::{BodyHtml, Feature, RenderPost};

use crate::html_check::assert_valid_page;
use crate::post::{part, render, render_on, site_without, sitemap_post};

/// The demo post with a figure whose original is 2400 pixels wide.
fn post_with_figure() -> RenderPost {
    let mut post = sitemap_post();
    post.body = BodyHtml::from_sanitised(
        "<p>A sitemap is a list of addresses.</p>\n\
         <h2 id=\"what-it-looks-like\">What it looks like</h2>\n\
         <figure>\n<img src=\"/blog/media/sitemap-800.png\" alt=\"A sitemap drawn as a tree of pages\" width=\"800\" height=\"450\" srcset=\"/blog/media/sitemap-800.png 800w\" sizes=\"100vw\" loading=\"lazy\" decoding=\"async\" />\n\
         <figcaption>\n<p>Every page you want found, in one list.</p>\n</figcaption>\n</figure>"
            .to_owned(),
    );
    post.originals = BTreeMap::from([(
        "/blog/media/sitemap-800.png".to_owned(),
        "/blog/media/sitemap-2400.png".to_owned(),
    )]);
    post
}

#[test]
fn a_reduced_image_links_to_its_largest_version() {
    let page = render("en", post_with_figure());
    assert_valid_page(&page);
    insta::assert_snapshot!(part(&page, "<figure>", "</figure>"));
}

#[test]
fn heading_links_and_enlarged_images_work_together() {
    let page = render("en", post_with_figure());
    assert!(page.contains("class=\"heading-link\" href=\"#what-it-looks-like\""));
    assert!(page.contains("<a class=\"enlarge\" href=\"/blog/media/sitemap-2400.png\">"));
}

#[test]
fn switched_off_images_do_not_link() {
    let page = render_on(&site_without(Feature::EnlargeImages), post_with_figure());
    assert_valid_page(&page);
    assert!(!page.contains("class=\"enlarge\""));
    assert!(page.contains("class=\"heading-link\""));
}
