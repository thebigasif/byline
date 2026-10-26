//! Video blocks and the structured data they give (task 1.7, KH-CON-5).

use std::time::Duration;

use khalid_content::{BlockKind, StructuredBlock, VideoBlock, parse_blocks, print_blocks, render};
use khalid_core::{Locale, SlugMode};

#[cfg(test)]
fn round_trip(markdown: &str) -> BlockKind {
    let blocks = parse_blocks(markdown).unwrap();
    assert_eq!(blocks.len(), 1, "{markdown:?} gave {blocks:?}");
    assert_eq!(print_blocks(&blocks), markdown);
    blocks[0].kind.clone()
}

#[cfg(test)]
fn rendered(markdown: &str) -> khalid_content::Rendered {
    render(markdown, &Locale::try_new("en").unwrap(), SlugMode::Ascii).unwrap()
}

#[cfg(test)]
fn error(markdown: &str) -> String {
    parse_blocks(markdown).unwrap_err().to_string()
}

const VIDEO: &str = "::video{src=\"/media/site-tour.mp4\" title=\"A tour of the site\" poster=\"/media/site-tour.jpg\" description=\"Where posts, pages and settings live.\" duration=\"2:05\"}\n";

#[cfg(test)]
fn tour() -> VideoBlock {
    VideoBlock {
        src: "/media/site-tour.mp4".to_owned(),
        title: "A tour of the site".to_owned(),
        poster: "/media/site-tour.jpg".to_owned(),
        description: Some("Where posts, pages and settings live.".to_owned()),
        duration: Some(Duration::from_secs(125)),
    }
}

#[test]
fn a_video_round_trips() {
    assert_eq!(round_trip(VIDEO), BlockKind::Video(tour()));
}

#[test]
fn a_video_needs_a_poster() {
    assert_eq!(
        error("::video{src=\"/media/a.mp4\" title=\"A\"}\n"),
        "line 1: a video needs poster, the still picture shown before it plays"
    );
}

#[test]
fn a_duration_is_minutes_and_seconds() {
    assert_eq!(
        error("::video{src=\"/a.mp4\" title=\"A\" poster=\"/a.jpg\" duration=\"2 minutes\"}\n"),
        "line 1: a video's duration is written as minutes:seconds, such as 2:05, or hours:minutes:seconds"
    );
}

#[test]
fn a_duration_too_long_to_count_is_refused() {
    assert!(
        error("::video{src=\"/a.mp4\" title=\"A\" poster=\"/a.jpg\" duration=\"99999999999999999:00:00\"}\n")
            .contains("duration is written as")
    );
}

#[test]
fn renders_a_video() {
    insta::assert_snapshot!(rendered(VIDEO).html);
}

#[test]
fn a_video_is_structured_data() {
    assert_eq!(rendered(VIDEO).structured, [StructuredBlock::Video(tour())]);
}
