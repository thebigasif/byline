//! Images and videos in the sitemap (task 1.13, KH-SEO-5).

#![cfg(test)]

mod common;
mod sitemap_xsd;

use std::time::Duration;

use common::{at, demo_site, internal_links_post, internal_links_post_de, url};
use khalid_seo::{SeoPage, Sitemap, Video, VideoSource};
use sitemap_xsd::{check_urlset, google_extensions};

fn walkthrough() -> Video {
    let mut video = Video::new(
        "Internal links in five minutes",
        "A short walk through adding internal links to an older post.",
        url("https://www.example.com/blog/media/internal-links-thumb.jpg"),
        VideoSource::Player(url("https://www.youtube-nocookie.com/embed/aqz-KE-bpKQ")),
    );
    video.duration = Some(Duration::from_secs(312));
    video.uploaded = Some(at(2026, 9, 12, 10));
    video
}

fn post_with_media() -> SeoPage {
    let mut post = internal_links_post();
    post.images = vec![
        url("https://www.example.com/blog/media/internal-links-map.png"),
        url("https://www.example.com/blog/media/anchor-text.png"),
    ];
    post.videos = vec![walkthrough()];
    post
}

#[test]
fn a_post_lists_its_images_and_videos() {
    let sitemap = Sitemap::from_pages(&demo_site(), &[post_with_media()]).unwrap();
    insta::assert_snapshot!(sitemap.to_xml());
}

#[test]
fn the_media_sitemap_is_valid_against_the_schemas() {
    let pages = [post_with_media(), internal_links_post_de()];
    let sitemap = Sitemap::from_pages(&demo_site(), &pages).unwrap();
    assert_eq!(check_urlset(&sitemap.to_xml(), google_extensions), Ok(2));
}

#[test]
fn a_post_without_media_lists_none() {
    let xml = Sitemap::from_pages(&demo_site(), &[internal_links_post_de()])
        .unwrap()
        .to_xml();
    assert!(!xml.contains("<image:image>"), "{xml}");
    assert!(!xml.contains("<video:video>"), "{xml}");
}

#[test]
fn a_long_video_description_is_cut_to_the_schema_limit() {
    let mut post = post_with_media();
    post.videos[0].description = "Links. ".repeat(400);
    let xml = Sitemap::from_pages(&demo_site(), &[post]).unwrap().to_xml();
    assert_eq!(check_urlset(&xml, google_extensions), Ok(1));
}

#[test]
fn a_page_lists_at_most_a_thousand_images() {
    let mut post = internal_links_post();
    post.images = (0..1200)
        .map(|n| url(&format!("https://www.example.com/blog/media/chart-{n}.png")))
        .collect();
    let sitemap = Sitemap::from_pages(&demo_site(), &[post]).unwrap();
    assert_eq!(sitemap.entries()[0].images.len(), 1000);
}
