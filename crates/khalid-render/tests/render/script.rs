//! The reader script: one small deferred file, checked by its hash, that
//! only adds polish. Every page reads completely without it.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use khalid_render::{ScriptTag, reader_script, reader_script_file};
use sha2::{Digest, Sha256};

use crate::common::demo_site;
use crate::html_check::assert_valid_page;
use crate::post::{part, render, render_on, sitemap_post};

/// The script's compressed size stays small, so it costs readers on slow
/// phones almost nothing. It is far under the 20 KB the stylesheet may use.
const BUDGET_BYTES: usize = 4 * 1024;

#[test]
fn a_page_loads_the_script_deferred_with_its_hash_and_words() {
    let script = reader_script().unwrap();
    let mut site = demo_site("en");
    site.script = Some(ScriptTag::new(&script, "/blog/assets/"));
    let page = render_on(&site, sitemap_post());
    assert_valid_page(&page);
    let tag = part(&page, "<script", "</script>");
    assert!(tag.contains(&format!("src=\"/blog/assets/{}\"", script.name())));
    assert!(tag.contains(&format!("integrity=\"{}\"", script.integrity())));
    assert!(tag.contains(" defer data-copied=\"Link copied\" data-close=\"Close\" data-previews>"));
}

#[test]
fn without_the_script_the_page_has_none_and_still_reads() {
    let page = render("en", sitemap_post());
    assert!(!page.contains("<script"));
    assert!(page.contains("<p>A sitemap is a file that lists the addresses"));
}

#[test]
fn the_script_file_without_compressed_copies_has_the_same_name_hash_and_bytes() {
    let script = reader_script().unwrap();
    let file = reader_script_file();
    assert_eq!(file.name, script.name());
    assert_eq!(file.integrity, script.integrity());
    assert_eq!(file.body, script.body());
    assert_eq!(
        ScriptTag::of_file(&file, "/blog/assets/"),
        ScriptTag::new(&script, "/blog/assets/")
    );
}

#[test]
fn the_script_is_a_hashed_file_whose_integrity_matches_its_bytes() {
    let script = reader_script().unwrap();
    let hash = script.name().strip_prefix("reader.").unwrap();
    assert_eq!(hash.len(), "3f2a9c1e.js".len(), "{}", script.name());
    assert_eq!(script.content_type(), "text/javascript; charset=utf-8");
    let expected = format!("sha256-{}", STANDARD.encode(Sha256::digest(script.body())));
    assert_eq!(script.integrity(), expected);
}

#[test]
fn the_script_is_small_and_writes_no_markup_from_strings() {
    let script = reader_script().unwrap();
    assert!(
        script.brotli().len() < BUDGET_BYTES,
        "brotli: {} bytes",
        script.brotli().len()
    );
    let text = std::str::from_utf8(script.body()).unwrap();
    for risky in [
        "innerHTML",
        "outerHTML",
        "insertAdjacentHTML",
        "document.write",
        "eval(",
        "onclick",
    ] {
        assert!(!text.contains(risky), "the script uses {risky}");
    }
}
