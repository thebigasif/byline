//! Settings come from `site.toml`, and every mistake is reported with the
//! file, the line and the field.

use std::path::Path;

use khalid_core::{Config, ConfigError, Locale, Mode, SlugMode};

const DEMO: &str = r#"
name = "Field Notes on SEO"
address = "https://www.example.com"
mount = "/blog/"
mode = "server"
default_language = "en"
languages = ["en", "de"]
"#;

fn file() -> &'static Path {
    Path::new("site.toml")
}

#[cfg(test)]
fn locale(tag: &str) -> Locale {
    Locale::try_new(tag).unwrap()
}

#[test]
fn reads_every_setting() {
    let config = Config::from_toml(file(), DEMO).unwrap();
    assert_eq!(config.name(), "Field Notes on SEO");
    assert_eq!(config.address().as_str(), "https://www.example.com/");
    assert_eq!(config.mount(), "/blog/");
    assert_eq!(config.mode(), Mode::Server);
    assert_eq!(config.slugs(), SlugMode::Ascii);
    assert_eq!(config.default_language(), &locale("en"));
    assert_eq!(config.languages(), [locale("en"), locale("de")]);
}

#[test]
fn loads_the_demo_site() {
    let site = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site"
    ));
    let config = Config::load(site).unwrap();
    assert_eq!(config.languages(), [locale("en"), locale("de")]);
}

/// Swaps one line of the demo file and returns the error.
#[cfg(test)]
fn error_with(old: &str, new: &str) -> ConfigError {
    let text = DEMO.replace(old, new);
    Config::from_toml(file(), &text).unwrap_err()
}

/// The line, field and file an error points at.
#[cfg(test)]
fn location(error: &ConfigError) -> (usize, &str, &Path) {
    match error {
        ConfigError::Field {
            path, line, field, ..
        }
        | ConfigError::Parse {
            path, line, field, ..
        } => (*line, field.as_str(), path.as_path()),
        other => panic!("not a field error: {other:?}"),
    }
}

#[test]
fn every_wrong_field_is_located() {
    let cases = [
        (
            r#"address = "https://www.example.com""#,
            r#"address = "www.example.com""#,
            3,
            "address",
        ),
        (r#"mode = "server""#, r#"mode = "hybrid""#, 5, "mode"),
        (
            r#"languages = ["en", "de"]"#,
            r#"languages = ["en", "english"]"#,
            7,
            "languages",
        ),
        (r#"mount = "/blog/""#, r#"mount = "blog""#, 4, "mount"),
        (
            r#"default_language = "en""#,
            r#"default_language = "fr""#,
            6,
            "default_language",
        ),
        (
            r#"mode = "server""#,
            "mode = \"server\"\ncolour = \"blue\"",
            6,
            "colour",
        ),
    ];
    for (old, new, line, field) in cases {
        let error = error_with(old, new);
        assert_eq!(
            location(&error),
            (line, field, file()),
            "replacing {old:?}: {error}"
        );
    }
}

#[test]
fn a_missing_field_is_named() {
    let error = error_with("name = \"Field Notes on SEO\"\n", "");
    assert_eq!(location(&error).1, "name", "{error}");
}

#[test]
fn a_missing_file_says_which_file() {
    let error = Config::load(Path::new("/no/such/site")).unwrap_err();
    assert!(matches!(&error, ConfigError::Read { path, .. } if path.ends_with("site.toml")));
}
