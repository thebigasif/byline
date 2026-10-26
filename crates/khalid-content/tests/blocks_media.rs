//! Audio, file download and gallery blocks (task 1.7, KH-CON-5).

use khalid_content::{BlockKind, GalleryImage, parse_blocks, print_blocks, render};
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

#[test]
fn an_audio_round_trips() {
    assert_eq!(
        round_trip("::audio{src=\"/media/episode-12.mp3\" title=\"Episode 12: internal links\"}\n"),
        BlockKind::Audio {
            src: "/media/episode-12.mp3".to_owned(),
            title: "Episode 12: internal links".to_owned(),
        }
    );
}

#[test]
fn a_file_round_trips() {
    assert_eq!(
        round_trip("::file{src=\"/media/link-audit.csv\" title=\"The link audit sheet\"}\n"),
        BlockKind::File {
            src: "/media/link-audit.csv".to_owned(),
            title: "The link audit sheet".to_owned(),
        }
    );
}

#[test]
fn a_gallery_round_trips() {
    assert_eq!(
        round_trip(
            ":::gallery\n![The editor with a post open](/media/editor.png)\n![The linter's findings](/media/lint.png)\n:::\n"
        ),
        BlockKind::Gallery {
            images: vec![
                GalleryImage {
                    src: "/media/editor.png".to_owned(),
                    alt: "The editor with a post open".to_owned(),
                },
                GalleryImage {
                    src: "/media/lint.png".to_owned(),
                    alt: "The linter's findings".to_owned(),
                },
            ],
        }
    );
}

#[test]
fn renders_an_audio_a_file_and_a_gallery() {
    let markdown = [
        "::audio{src=\"/media/episode-12.mp3\" title=\"Episode 12: internal links\"}\n",
        "::file{src=\"/media/link-audit.csv\" title=\"The link audit sheet\"}\n",
        ":::gallery\n![The editor with a post open](/media/editor.png)\n![The linter's findings](/media/lint.png)\n:::\n",
    ]
    .join("\n");
    insta::assert_snapshot!(rendered(&markdown).html);
}

#[test]
fn gallery_images_join_the_images() {
    let page = rendered(":::gallery\n![The editor](/media/editor.png)\n:::\n");
    let images: Vec<&str> = page.images.iter().map(|image| image.src.as_str()).collect();
    assert_eq!(images, ["/media/editor.png"]);
}

#[test]
fn a_gallery_holds_only_images_with_alt_text() {
    assert_eq!(
        error("Intro.\n\n:::gallery\n![](/media/editor.png)\n:::\n"),
        "line 4: each line of a gallery is an image with alt text, such as ![The editor](/media/editor.png)"
    );
}

#[test]
fn an_audio_needs_a_title() {
    assert_eq!(
        error("::audio{src=\"/media/episode-12.mp3\"}\n"),
        "line 1: an audio block needs title, the words shown above the player"
    );
}
