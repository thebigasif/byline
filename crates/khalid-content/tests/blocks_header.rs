//! Header blocks: a banner with a picture and words (task 1.7, KH-CON-5).

use khalid_content::{BlockKind, parse_blocks, print_blocks, render};
use khalid_core::{Locale, SlugMode};

const HEADER: &str = "\
:::header{image=\"/media/links-cover.jpg\" alt=\"A map of pages joined by lines\"}
## Part two: linking old posts

Ten minutes a week keeps every post one click from a new one.
:::
";

#[test]
fn a_header_round_trips() {
    let blocks = parse_blocks(HEADER).unwrap();
    assert_eq!(print_blocks(&blocks), HEADER);
    assert_eq!(
        blocks[0].kind,
        BlockKind::Header {
            image: Some("/media/links-cover.jpg".to_owned()),
            alt: Some("A map of pages joined by lines".to_owned()),
        }
    );
}

#[test]
fn renders_a_header_whose_heading_joins_the_contents() {
    let page = render(HEADER, &Locale::try_new("en").unwrap(), SlugMode::Ascii).unwrap();
    assert_eq!(page.headings[0].id, "part-two-linking-old-posts");
    assert_eq!(page.images[0].src, "/media/links-cover.jpg");
    insta::assert_snapshot!(page.html);
}

#[test]
fn a_header_picture_needs_alt_text() {
    assert_eq!(
        parse_blocks(":::header{image=\"/media/cover.jpg\"}\n## Part two\n:::\n")
            .unwrap_err()
            .to_string(),
        "line 1: a header's picture needs alt, the text that describes it to people who cannot see it"
    );
}
