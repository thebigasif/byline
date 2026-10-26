//! Built assets: a name that changes with the content, and gzip and Brotli
//! copies that decompress to the same bytes.

use std::io::Read;

use khalid_render::Asset;

#[cfg(test)]
fn css(text: &str) -> Asset {
    Asset::build(
        "site",
        "css",
        "text/css; charset=utf-8",
        text.as_bytes().to_vec(),
    )
    .unwrap()
}

#[test]
fn the_name_carries_a_hash_of_the_content() {
    let asset = css("body { color: #1f2328; }");
    let name = asset.name();
    let hash = name
        .strip_prefix("site.")
        .unwrap()
        .strip_suffix(".css")
        .unwrap();
    assert_eq!(hash.len(), 8, "{name}");
    assert!(
        hash.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "{name}"
    );
    assert_eq!(asset.content_type(), "text/css; charset=utf-8");
}

#[test]
fn the_same_content_gets_the_same_name_and_a_change_gets_a_new_one() {
    let first = css("body { color: #1f2328; }");
    assert_eq!(
        first,
        css("body { color: #1f2328; }"),
        "builds are deterministic"
    );
    assert_ne!(first.name(), css("body { color: #000000; }").name());
}

#[test]
fn the_compressed_copies_hold_the_same_bytes() {
    let text = "a { color: var(--color-link); }\n".repeat(200);
    let asset = css(&text);
    let mut from_gzip = Vec::new();
    flate2::read::GzDecoder::new(asset.gzip())
        .read_to_end(&mut from_gzip)
        .unwrap();
    let mut from_brotli = Vec::new();
    brotli::Decompressor::new(asset.brotli(), 4096)
        .read_to_end(&mut from_brotli)
        .unwrap();
    assert_eq!(from_gzip, text.as_bytes());
    assert_eq!(from_brotli, text.as_bytes());
    assert_eq!(asset.body(), text.as_bytes());
    assert!(asset.brotli().len() < asset.gzip().len());
    assert!(asset.gzip().len() < asset.body().len() / 10);
}
