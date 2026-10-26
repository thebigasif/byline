//! The one public stylesheet: built from the tokens, under its budget, and
//! covering every class the default templates use.

use std::fs;
use std::path::PathBuf;

use khalid_render::{Tokens, stylesheet};

/// RULES.md BL-WEB-7 and docs/standards/performance.md: the public CSS stays
/// under 20 KB compressed.
const BUDGET_BYTES: usize = 20 * 1024;

#[test]
fn the_stylesheet_stays_under_twenty_kilobytes_compressed() {
    let css = stylesheet(&Tokens::default()).unwrap();
    assert!(
        css.brotli().len() < BUDGET_BYTES,
        "brotli: {} bytes",
        css.brotli().len()
    );
    assert!(
        css.gzip().len() < BUDGET_BYTES,
        "gzip: {} bytes",
        css.gzip().len()
    );
    println!(
        "public CSS: {} bytes, {} gzip, {} brotli",
        css.body().len(),
        css.gzip().len(),
        css.brotli().len()
    );
}

#[test]
fn the_tokens_come_first_and_the_rules_read_them() {
    let tokens = Tokens::from_toml("[colors]\nlink = \"#b3261e\"").unwrap();
    let css = stylesheet(&tokens).unwrap();
    let text = std::str::from_utf8(css.body()).unwrap();
    assert!(text.starts_with(":root {\n  --color-background: #ffffff;"));
    assert!(text.contains("--color-link: #b3261e;"));
    assert!(text.contains("color: var(--color-link);"));
    assert!(!text.contains("/*"), "comments are removed");
    let hash = css.name().strip_prefix("site.").unwrap();
    assert_eq!(hash.split_once('.').unwrap().1, "css", "{}", css.name());
}

#[test]
fn new_tokens_make_a_new_file_name() {
    let light = stylesheet(&Tokens::default()).unwrap();
    let red = stylesheet(&Tokens::from_toml("[colors]\nlink = \"#b3261e\"").unwrap()).unwrap();
    assert_ne!(light.name(), red.name());
}

/// Whether `css` has `.class` as a whole class name, so `.post` does not
/// count because `.post-meta` exists.
#[cfg(test)]
fn has_selector(css: &str, class: &str) -> bool {
    let needle = format!(".{class}");
    css.match_indices(&needle).any(|(at, _)| {
        let after = css
            .get(at + needle.len()..)
            .and_then(|rest| rest.chars().next());
        !after.is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_')
    })
}

#[test]
fn every_class_a_default_template_uses_is_styled() {
    let css = String::from_utf8(stylesheet(&Tokens::default()).unwrap().body().to_vec()).unwrap();
    let mut folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    folder.push("templates");
    for entry in fs::read_dir(&folder).unwrap() {
        let path = entry.unwrap().path();
        let template = fs::read_to_string(&path).unwrap();
        for attribute in template.split("class=\"").skip(1) {
            let classes = attribute.split('"').next().unwrap();
            for class in classes.split_whitespace() {
                assert!(
                    has_selector(&css, class),
                    ".{class} from {}",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn the_page_is_light_and_mirrors_for_right_to_left() {
    let css = String::from_utf8(stylesheet(&Tokens::default()).unwrap().body().to_vec()).unwrap();
    assert!(css.contains("color-scheme: light;"));
    assert!(!css.contains("prefers-color-scheme"), "no dark mode");
    assert!(
        !css.contains("margin-left") && !css.contains("padding-left"),
        "logical properties only"
    );
    assert!(css.contains("@media print"));
    assert!(css.contains("prefers-reduced-motion"));
}
