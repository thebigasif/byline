//! The size measures read what the pages and the admin load first, and
//! leave out an admin that was never built.

use std::fs;
use std::path::{Path, PathBuf};

use khalid_perf::{Linked, Sizes, linked};

#[cfg(test)]
fn write(dir: &Path, name: &str, bytes: &[u8]) {
    let mut path = PathBuf::from(dir);
    path.push(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// `len` bytes gzip cannot shrink, so a counted file would show.
#[cfg(test)]
fn noise(len: usize) -> Vec<u8> {
    let mut state: u32 = 0x9e37_79b9;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state.to_le_bytes()[0]
        })
        .collect()
}

const HOME: &str = r#"<head><link rel="stylesheet" href="/blog/assets/site.1a2b3c4d.css">
<link rel="alternate" href="/blog/feed.xml"></head>"#;

const ADMIN: &str = r#"<head><script type="module" crossorigin src="/admin/assets/index-a1.js"></script>
<link rel="modulepreload" crossorigin href="/admin/assets/vendor-b2.js">
<link rel="stylesheet" href="/admin/assets/index-c3.css"></head>"#;

#[test]
fn the_stylesheets_and_first_scripts_are_read_from_the_page() {
    assert_eq!(
        linked(HOME, Linked::Stylesheet),
        vec!["/blog/assets/site.1a2b3c4d.css"]
    );
    assert_eq!(
        linked(ADMIN, Linked::Script),
        vec!["/admin/assets/index-a1.js", "/admin/assets/vendor-b2.js"]
    );
}

#[test]
fn the_css_is_the_size_of_the_brotli_copy_the_build_wrote() {
    let build = tempfile::tempdir().unwrap();
    write(build.path(), "index.html", HOME.as_bytes());
    write(build.path(), "assets/site.1a2b3c4d.css", &[b'a'; 4096]);
    write(build.path(), "assets/site.1a2b3c4d.css.br", &[0; 2048]);
    let none = tempfile::tempdir().unwrap();
    let sizes = Sizes::measure(build.path(), none.path()).unwrap();
    assert_eq!(sizes.public_css_kb(), 2.0);
    assert_eq!(sizes.admin_js_kb(), None, "no admin was built");
    assert_eq!(sizes.to_json(), "{\"public_css_compressed_kb\": 2.00}\n");
}

#[test]
fn the_admin_counts_its_first_scripts_compressed_and_nothing_else() {
    let build = tempfile::tempdir().unwrap();
    write(build.path(), "index.html", HOME.as_bytes());
    write(build.path(), "assets/site.1a2b3c4d.css", b"body{}");
    write(build.path(), "assets/site.1a2b3c4d.css.br", &[0; 1024]);
    let dist = tempfile::tempdir().unwrap();
    write(dist.path(), "index.html", ADMIN.as_bytes());
    write(dist.path(), "assets/index-a1.js", b"export const a = 1;\n");
    write(dist.path(), "assets/vendor-b2.js", b"export const b = 2;\n");
    write(dist.path(), "assets/lazy-d4.js", &noise(100_000));
    let sizes = Sizes::measure(build.path(), dist.path()).unwrap();
    let kb = sizes.admin_js_kb().unwrap();
    assert!(kb > 0.0 && kb < 1.0, "the lazy chunk is not counted: {kb}");
    assert!(
        sizes
            .to_json()
            .contains(", \"admin_initial_js_compressed_kb\": ")
    );
}

#[test]
fn a_link_that_climbs_out_of_the_build_is_not_followed() {
    let outside = tempfile::tempdir().unwrap();
    write(outside.path(), "secret.css", b"body{}");
    write(outside.path(), "secret.css.br", &[0; 512]);
    write(
        outside.path(),
        "build/index.html",
        br#"<link rel="stylesheet" href="/blog/../secret.css">"#,
    );
    let mut build = PathBuf::from(outside.path());
    build.push("build");
    let error = Sizes::measure(&build, outside.path()).unwrap_err();
    assert!(error.to_string().contains("is missing"), "{error}");
}

#[test]
fn a_stylesheet_without_its_brotli_copy_is_an_error() {
    let build = tempfile::tempdir().unwrap();
    write(build.path(), "index.html", HOME.as_bytes());
    write(build.path(), "assets/site.1a2b3c4d.css", b"body{}");
    let none = tempfile::tempdir().unwrap();
    let error = Sizes::measure(build.path(), none.path()).unwrap_err();
    assert!(
        error.to_string().contains("site.1a2b3c4d.css.br"),
        "{error}"
    );
}
