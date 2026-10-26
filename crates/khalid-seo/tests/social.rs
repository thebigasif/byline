//! Open Graph and Twitter card tags (task 1.13, KH-SEO-2).

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, locale, path, url};
use khalid_seo::{PageKind, SeoPage, ShareImage, TwitterHandle, UrlPath, head_tags};

fn default_image() -> ShareImage {
    ShareImage::new(
        url("https://www.example.com/blog/media/share-default.png"),
        1200,
        630,
        "Field Notes on SEO, plain advice on search",
    )
}

#[test]
fn a_post_gets_article_tags_and_the_default_share_image() {
    let mut site = demo_site();
    site.share_image = Some(default_image());
    site.twitter_site = Some(TwitterHandle::try_new("@serpdad").unwrap());
    let tags = head_tags(&site, &internal_links_post()).unwrap();
    insta::assert_snapshot!(tags.to_html());
}

#[test]
fn a_page_image_overrides_the_default() {
    let mut site = demo_site();
    site.share_image = Some(default_image());
    let mut post = internal_links_post();
    post.share_image = Some(ShareImage::new(
        url("https://www.example.com/blog/media/internal-links.png"),
        1600,
        900,
        "A site map with arrows between pages",
    ));
    let html = head_tags(&site, &post).unwrap().to_html();
    assert!(
        html.contains(
            r#"<meta property="og:image" content="https://www.example.com/blog/media/internal-links.png">"#
        ),
        "{html}"
    );
    assert!(!html.contains("share-default.png"), "{html}");
}

#[test]
fn a_page_with_no_image_anywhere_gets_a_summary_card() {
    let html = head_tags(&demo_site(), &internal_links_post())
        .unwrap()
        .to_html();
    assert!(
        html.contains(r#"<meta name="twitter:card" content="summary">"#),
        "{html}"
    );
    assert!(!html.contains("og:image"), "{html}");
}

#[test]
fn the_home_page_is_a_website_named_after_the_site() {
    let home = SeoPage::new(PageKind::Home, locale("en"), UrlPath::root(), "");
    let html = head_tags(&demo_site(), &home).unwrap().to_html();
    assert!(
        html.contains(r#"<meta property="og:type" content="website">"#),
        "{html}"
    );
    assert!(
        html.contains(r#"<meta property="og:title" content="Field Notes on SEO">"#),
        "{html}"
    );
    assert!(!html.contains("article:"), "{html}");
}

#[test]
fn og_url_follows_the_canonical_and_og_locale_uses_an_underscore() {
    let mut post = internal_links_post_de();
    post.language = locale("pt-BR");
    post.path = path("/links-internos/");
    let html = head_tags(&demo_site(), &post).unwrap().to_html();
    assert!(
        html.contains(
            r#"<meta property="og:url" content="https://www.example.com/blog/pt-br/links-internos/">"#
        ),
        "{html}"
    );
    assert!(
        html.contains(r#"<meta property="og:locale" content="pt_BR">"#),
        "{html}"
    );
}

#[test]
fn a_twitter_handle_is_checked() {
    assert!(TwitterHandle::try_new("serpdad").is_ok());
    assert_eq!(
        TwitterHandle::try_new("serpdad").unwrap().as_str(),
        "@serpdad"
    );
    assert!(TwitterHandle::try_new("@").is_err());
    assert!(TwitterHandle::try_new("@has space").is_err());
    assert!(TwitterHandle::try_new("@fifteen_letters").is_ok());
    assert!(TwitterHandle::try_new("@sixteen_letters1").is_err());
}
