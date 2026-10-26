//! Responsive images: AVIF and WebP sources, and a fallback `img` that always
//! has `srcset`, `sizes`, `width`, `height` and `alt` (BL-WEB-5).

use khalid_render::{ImageCandidate, ImageSet, Loading, PageBody, PageKind, RenderImage, Renderer};

use crate::common::{demo_site, meta};
use crate::html_check::assert_valid_page;
use crate::post::sitemap_post;

fn candidate(url: &str, width: u32) -> ImageCandidate {
    ImageCandidate {
        url: url.to_owned(),
        width,
    }
}

fn set(stem: &str, extension: &str) -> ImageSet {
    ImageSet::new(
        candidate(&format!("/blog/media/{stem}-1600.{extension}"), 1600),
        vec![candidate(
            &format!("/blog/media/{stem}-800.{extension}"),
            800,
        )],
    )
}

pub(crate) fn sitemap_image(loading: Loading) -> RenderImage {
    RenderImage {
        alt: "A sitemap drawn as a tree of pages".to_owned(),
        caption: Some("Every page you want found, in one list.".to_owned()),
        width: 1600,
        height: 900,
        sizes: "(min-width: 48rem) 48rem, 100vw".to_owned(),
        avif: Some(set("3f2a9c1e", "avif")),
        webp: Some(set("3f2a9c1e", "webp")),
        fallback: set("3f2a9c1e", "jpg"),
        loading,
    }
}

/// Lena Vogt's photo, square and small, as a byline shows it.
pub(crate) fn avatar_image() -> RenderImage {
    RenderImage {
        alt: String::new(),
        caption: None,
        width: 1600,
        height: 1600,
        sizes: "2.75rem".to_owned(),
        avif: Some(set("lena-vogt", "avif")),
        webp: None,
        fallback: set("lena-vogt", "jpg"),
        loading: Loading::Lazy,
    }
}

fn render_post_with(image: RenderImage) -> String {
    let mut post = sitemap_post();
    post.image = Some(image);
    let meta = meta(&post.title, "/blog/what-a-sitemap-does/", PageKind::Article);
    Renderer::new()
        .unwrap()
        .render(&demo_site("en"), &meta, &PageBody::Post(Box::new(post)))
        .unwrap()
}

#[test]
fn an_image_set_is_sorted_by_width_with_one_file_per_width() {
    let set = ImageSet::new(
        candidate("/b.webp", 960),
        vec![candidate("/a.webp", 480), candidate("/c.webp", 960)],
    );
    assert_eq!(set.to_srcset(), "/a.webp 480w, /b.webp 960w");
    assert_eq!(set.widest().unwrap().url, "/b.webp");
}

#[test]
fn a_lead_image_is_a_picture_with_every_format() {
    let page = render_post_with(sitemap_image(Loading::Eager));
    assert_valid_page(&page);
    insta::assert_snapshot!(
        page.split("<figure")
            .nth(1)
            .unwrap()
            .split("</figure>")
            .next()
            .unwrap()
    );
}

#[test]
fn the_lead_image_loads_at_once_and_others_wait() {
    let eager = render_post_with(sitemap_image(Loading::Eager));
    assert!(eager.contains(r#"fetchpriority="high""#));
    assert!(!eager.contains(r#"loading="lazy""#));
    let lazy = render_post_with(sitemap_image(Loading::Lazy));
    assert!(lazy.contains(r#"loading="lazy""#));
    assert!(!lazy.contains("fetchpriority"));
}

#[test]
fn an_image_with_only_its_original_format_still_reserves_its_space() {
    let mut image = sitemap_image(Loading::Lazy);
    image.avif = None;
    image.webp = None;
    image.caption = None;
    let page = render_post_with(image);
    assert_valid_page(&page);
    assert!(!page.contains("<source"));
    assert!(!page.contains("<figcaption"));
    assert!(page.contains(r#"width="1600" height="900""#));
}
