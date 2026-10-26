//! The video and review blocks (task 1.14, KH-SEO-3).

#![cfg(test)]

mod common;

use std::time::Duration;

use common::{at, demo_site, internal_links_post, internal_links_post_de, url};
use khalid_seo::{
    AggregateRating, Rating, Review, SchemaBlock, SchemaGraph, SeoError, Video, VideoSource,
    page_graph,
};

fn video() -> Video {
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

fn review() -> Review {
    let mut review = Review::new(
        "SoftwareApplication",
        "Screaming Frog SEO Spider",
        Rating::new(4.5, 5.0).unwrap(),
        "Asif",
    );
    review.body =
        String::from("Finds every orphan page in minutes. The interface takes a day to learn.");
    review.aggregate = Some(AggregateRating::new(Rating::new(4.6, 5.0).unwrap(), 212));
    review
}

#[test]
fn a_post_with_a_video_and_a_review_adds_both() {
    let mut post = internal_links_post();
    post.blocks = vec![SchemaBlock::Video(video()), SchemaBlock::Review(review())];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let blocks = graph.nodes()[graph.nodes().len() - 2..].to_vec();
    assert_eq!(blocks[0].kind(), "VideoObject");
    assert_eq!(blocks[1].kind(), "Review");
    insta::assert_snapshot!(SchemaGraph::new(blocks).to_json());
}

#[test]
fn a_video_file_and_player_are_both_named() {
    let mut both = video();
    both.source = VideoSource::FileAndPlayer {
        file: url("https://www.example.com/blog/media/internal-links.mp4"),
        player: url("https://www.youtube-nocookie.com/embed/aqz-KE-bpKQ"),
    };
    let mut post = internal_links_post_de();
    post.blocks = vec![SchemaBlock::Video(both)];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let node = graph.nodes().last().unwrap();
    assert!(node.get("contentUrl").is_some() && node.get("embedUrl").is_some());
}

#[test]
fn a_rating_must_sit_between_zero_and_its_best() {
    assert!(Rating::new(4.5, 5.0).is_ok());
    assert!(Rating::new(10.0, 10.0).is_ok());
    for (value, best) in [(5.5, 5.0), (-1.0, 5.0), (3.0, 0.0), (f64::NAN, 5.0)] {
        assert!(
            matches!(Rating::new(value, best), Err(SeoError::Rating { .. })),
            "{value} of {best}"
        );
    }
}
