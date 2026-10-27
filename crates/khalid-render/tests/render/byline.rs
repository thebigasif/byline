//! The byline: each author's photo, name and job title, in order, then the
//! people who reviewed the post. Each part switches off on its own.

use khalid_render::Feature;

use crate::html_check::assert_valid_page;
use crate::image::avatar_image;
use crate::post::{link, part, person, render, render_on, site_without, sitemap_post};

#[test]
fn the_byline_shows_each_author_with_photo_and_role_then_the_reviewers() {
    let mut post = sitemap_post();
    post.authors[0].image = Some(avatar_image());
    post.authors.push(person("Asif", "asif", None));
    post.reviewers = vec![link("Mara Lind", "/blog/authors/mara-lind/")];
    let page = render("en", post);
    assert_valid_page(&page);
    insta::assert_snapshot!(part(&page, "<div class=\"post-meta\">", "</div>"));
}

#[test]
fn several_authors_are_listed_in_order() {
    let mut post = sitemap_post();
    post.authors.push(person("Asif", "asif", None));
    let page = render("en", post);
    let lena = page.find(r#"href="/blog/authors/lena-vogt/">Lena Vogt</a>"#);
    let asif = page.find(r#"href="/blog/authors/asif/">Asif</a>"#);
    assert!(lena.unwrap() < asif.unwrap());
}

#[test]
fn without_the_byline_feature_the_authors_are_plain_names() {
    let mut post = sitemap_post();
    post.authors[0].image = Some(avatar_image());
    post.authors.push(person("Asif", "asif", None));
    let page = render_on(&site_without(Feature::Byline), post);
    assert_valid_page(&page);
    assert!(page.contains(
        r#"<address>By <a rel="author" href="/blog/authors/lena-vogt/">Lena Vogt</a>, <a rel="author" href="/blog/authors/asif/">Asif</a></address>"#
    ));
    let byline = part(&page, "<div class=\"post-meta\">", "</div>");
    assert!(!byline.contains("Technical SEO editor"));
    assert!(!byline.contains("<picture"));
}

#[test]
fn reviewers_show_only_when_there_are_some_and_the_feature_is_on() {
    let mut post = sitemap_post();
    post.reviewers = vec![link("Mara Lind", "/blog/authors/mara-lind/")];
    let shown = render("en", post.clone());
    assert!(shown.contains(
        r#"<p class="reviewed-by">Reviewed by <a href="/blog/authors/mara-lind/">Mara Lind</a></p>"#
    ));
    let off = render_on(&site_without(Feature::ReviewedBy), post);
    assert!(!off.contains("Reviewed by"));
    assert!(!render("en", sitemap_post()).contains("Reviewed by"));
}

#[test]
fn reviewers_read_in_the_page_language() {
    let mut post = sitemap_post();
    post.reviewers = vec![link("Mara Lind", "/blog/de/authors/mara-lind/")];
    assert!(render("de", post).contains("Geprüft von <a"));
}
