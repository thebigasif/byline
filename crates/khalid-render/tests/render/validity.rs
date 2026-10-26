//! One snapshot per page type, each checked for validity first, and the share
//! card every page carries.

use khalid_render::{PageBody, PageKind, RenderSite, Renderer, ShareImage};

use crate::common::{demo_site, meta, url};
use crate::html_check::assert_valid_page;
use crate::pages::render_not_found;

fn render_glossary(site: &RenderSite) -> String {
    let meta = meta("Glossary", "/blog/glossary/", PageKind::Website);
    Renderer::new()
        .unwrap()
        .render(site, &meta, &PageBody::NotFound)
        .unwrap()
}

#[test]
fn not_found_page_in_english() {
    let page = render_not_found("en");
    assert_valid_page(&page);
    insta::assert_snapshot!(page);
}

#[test]
fn not_found_page_in_german_is_valid() {
    assert_valid_page(&render_not_found("de"));
}

#[test]
fn a_page_with_menus_and_languages_is_valid() {
    assert_valid_page(&render_glossary(&demo_site("en")));
}

#[test]
fn a_page_without_its_own_image_shares_the_site_image() {
    let page = render_glossary(&demo_site("en"));
    assert!(page.contains(
        r#"<meta property="og:image" content="https://www.example.com/blog/assets/share.png">"#
    ));
    assert!(page.contains(r#"<meta property="og:image:width" content="1200">"#));
    assert!(page.contains(r#"<meta name="twitter:card" content="summary_large_image">"#));
}

#[test]
fn a_page_image_wins_over_the_site_image() {
    let mut meta = meta(
        "Fixing slow pages",
        "/blog/fixing-slow-pages/",
        PageKind::Article,
    );
    meta.image = Some(ShareImage {
        url: url("https://www.example.com/blog/fixing-slow-pages/speed.png"),
        alt: "A page load timeline".to_owned(),
        width: 1600,
        height: 900,
    });
    let page = Renderer::new()
        .unwrap()
        .render(&demo_site("en"), &meta, &PageBody::NotFound)
        .unwrap();
    assert!(page.contains("fixing-slow-pages/speed.png"));
    assert!(!page.contains("assets/share.png"));
    assert!(page.contains(r#"<meta property="og:type" content="article">"#));
}

#[test]
fn a_site_without_a_share_image_gets_a_small_card() {
    let mut site = demo_site("en");
    site.share_image = None;
    let page = render_glossary(&site);
    assert!(!page.contains("og:image"));
    assert!(page.contains(r#"<meta name="twitter:card" content="summary">"#));
}
