//! Embed blocks and the provider allowlist (task 1.7, KH-CON-5).

use khalid_content::{
    BlockKind, EMBED_PROVIDERS, EmbedProvider, parse_blocks, print_blocks, render,
};
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
fn an_embed_round_trips() {
    assert_eq!(
        round_trip(
            "::embed{src=\"https://www.youtube.com/watch?v=dQw4w9WgXcQ\" title=\"How search works\"}\n"
        ),
        BlockKind::Embed {
            src: url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            title: "How search works".to_owned(),
        }
    );
}

#[test]
fn renders_each_provider_in_a_frame_and_any_other_address_as_a_link() {
    insta::assert_snapshot!(html(
        "\
::embed{src=\"https://www.youtube.com/watch?v=dQw4w9WgXcQ\" title=\"How search works\"}

::embed{src=\"https://youtu.be/dQw4w9WgXcQ\" title=\"How search works\"}

::embed{src=\"https://vimeo.com/76979871\" title=\"A short film\"}

::embed{src=\"https://open.spotify.com/episode/4rOoJ6Egrf8K2IrywzwOMk\" title=\"An SEO podcast\"}

::embed{src=\"https://soundcloud.com/forss/flickermood\" title=\"Flickermood\"}

::embed{src=\"https://codepen.io/chriscoyier/pen/gfdDu\" title=\"A layout demo\"}

::embed{src=\"https://www.loom.com/share/0281766fa2d04bb788eaf19e65135184\" title=\"A site tour\"}

::embed{src=\"https://example.com/talk\" title=\"A talk elsewhere\"}
"
    ));
}

#[test]
fn an_embed_joins_the_links() {
    let rendered = render(
        "::embed{src=\"https://vimeo.com/1\" title=\"A film\"}\n",
        &Locale::try_new("en").unwrap(),
        SlugMode::Ascii,
    )
    .unwrap();
    let links: Vec<(&str, &str)> = rendered
        .links
        .iter()
        .map(|link| (link.href.as_str(), link.text.as_str()))
        .collect();
    assert_eq!(links, [("https://vimeo.com/1", "A film")]);
}

#[test]
fn the_allowlist_names_every_frame_origin_once() {
    let origins: Vec<&str> = EMBED_PROVIDERS
        .iter()
        .map(|provider| provider.frame_origin())
        .collect();
    assert_eq!(
        origins,
        [
            "https://www.youtube-nocookie.com",
            "https://player.vimeo.com",
            "https://open.spotify.com",
            "https://w.soundcloud.com",
            "https://codepen.io",
            "https://www.loom.com",
        ]
    );
    assert_eq!(EMBED_PROVIDERS[0], EmbedProvider::YouTube);
}

#[test]
fn an_embed_needs_a_title() {
    assert_eq!(
        error("::embed{src=\"https://vimeo.com/1\"}\n"),
        "line 1: an embed needs title, the name a screen reader says for the player"
    );
}
