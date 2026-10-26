//! Bookmark blocks, links to other sites drawn as cards (task 1.7, KH-CON-5).

use khalid_content::{BlockKind, parse_blocks, print_blocks, render};
use khalid_core::{AbsoluteUrl, Locale, SlugMode};

#[cfg(test)]
fn round_trip(markdown: &str) -> BlockKind {
    let blocks = parse_blocks(markdown).unwrap();
    assert_eq!(blocks.len(), 1, "{markdown:?} gave {blocks:?}");
    assert_eq!(print_blocks(&blocks), markdown);
    blocks[0].kind.clone()
}

#[cfg(test)]
fn html(markdown: &str) -> String {
    render(markdown, &Locale::try_new("en").unwrap(), SlugMode::Ascii)
        .unwrap()
        .html
}

#[cfg(test)]
fn error(markdown: &str) -> String {
    parse_blocks(markdown).unwrap_err().to_string()
}

#[cfg(test)]
fn url(text: &str) -> AbsoluteUrl {
    AbsoluteUrl::parse(text).unwrap()
}

#[test]
fn a_bare_bookmark_round_trips() {
    assert_eq!(
        round_trip("::bookmark{href=\"https://web.dev/articles/vitals\"}\n"),
        BlockKind::Bookmark {
            href: url("https://web.dev/articles/vitals"),
            title: None,
            description: None,
            image: None,
        }
    );
}

#[test]
fn a_bookmark_filled_at_publish_round_trips() {
    let markdown = "::bookmark{href=\"https://web.dev/articles/vitals\" title=\"Web Vitals\" description=\"Quality signals for a good experience.\" image=\"https://web.dev/vitals.png\"}\n";
    assert_eq!(
        round_trip(markdown),
        BlockKind::Bookmark {
            href: url("https://web.dev/articles/vitals"),
            title: Some("Web Vitals".to_owned()),
            description: Some("Quality signals for a good experience.".to_owned()),
            image: Some(url("https://web.dev/vitals.png")),
        }
    );
}

#[test]
fn renders_bookmarks_as_links_and_cards() {
    insta::assert_snapshot!(html(
        "\
::bookmark{href=\"https://web.dev/articles/vitals\"}

::bookmark{href=\"https://web.dev/articles/vitals\" title=\"Web Vitals\" description=\"Quality signals for a good experience.\" image=\"https://web.dev/vitals.png\"}
"
    ));
}

#[test]
fn a_bookmark_refuses_a_script_address() {
    assert_eq!(
        error("::bookmark{href=\"javascript:alert(1)\"}\n"),
        "line 1: a bookmark's href is wrong: the address uses javascript:, and only http: and https: are allowed"
    );
}

#[test]
fn a_bookmark_joins_the_links() {
    let rendered = render(
        "::bookmark{href=\"https://web.dev/\" title=\"web.dev\"}\n",
        &Locale::try_new("en").unwrap(),
        SlugMode::Ascii,
    )
    .unwrap();
    assert_eq!(rendered.links.len(), 1);
    assert_eq!(rendered.links[0].href, "https://web.dev/");
    assert_eq!(rendered.links[0].text, "web.dev");
    assert!(!rendered.links[0].internal);
}
