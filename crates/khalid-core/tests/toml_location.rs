//! The shared helpers that locate a mistake in a TOML file.

use khalid_core::toml_location::{field_of, line_of, line_of_key};

const TEXT: &str =
    "name = \"Field Notes\"\nmode = \"server\"\n\n[[collections]]\nslug = \"guides\"\n";

#[test]
fn counts_lines_from_one() {
    assert_eq!(line_of(TEXT, 0), 1);
    assert_eq!(line_of(TEXT, TEXT.find("mode").unwrap()), 2);
    assert_eq!(line_of(TEXT, TEXT.find("slug").unwrap()), 5);
}

#[test]
fn finds_the_line_a_key_is_on() {
    assert_eq!(line_of_key(TEXT, "mode"), 2);
    assert_eq!(line_of_key(TEXT, "slug"), 5);
    assert_eq!(line_of_key(TEXT, "missing"), 1);
}

#[test]
fn names_the_field_an_error_is_about() {
    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    #[expect(dead_code, reason = "only the error is read")]
    struct Site {
        name: String,
        mode: u8,
    }
    let unknown = "name = \"a\"\ncolour = \"blue\"\n";
    let error = toml::from_str::<Site>(unknown).unwrap_err();
    assert_eq!(field_of(unknown, &error), "colour");
    let wrong = "name = \"a\"\nmode = \"server\"\n";
    let error = toml::from_str::<Site>(wrong).unwrap_err();
    assert_eq!(field_of(wrong, &error), "mode");
}
