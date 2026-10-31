//! Settings come from `site.toml` and the environment. Every refusal has its
//! own test, named for the mistake, with the exact message a person reads.

use std::path::Path;

use khalid_core::{Config, ConfigError, Environment, Locale, Mode, SlugMode};

const DEMO: &str = r#"name = "Field Notes on SEO"
address = "https://www.example.com"
mount = "/blog/"
mode = "server"
default_language = "en"
languages = ["en", "de"]
"#;

#[cfg(test)]
fn locale(tag: &str) -> Locale {
    Locale::try_new(tag).unwrap()
}

/// Reads the demo text with `old` replaced by `new`, and returns the message.
#[cfg(test)]
fn message_with(old: &str, new: &str) -> String {
    assert!(DEMO.contains(old), "the demo has no {old:?}");
    let text = DEMO.replacen(old, new, 1);
    Config::from_toml(Path::new("site.toml"), &text, &Environment::empty())
        .unwrap_err()
        .to_string()
}

#[test]
fn reads_every_setting() {
    let config = Config::from_toml(Path::new("site.toml"), DEMO, &Environment::empty()).unwrap();
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
    let config = Config::load(site, &Environment::empty()).unwrap();
    assert_eq!(config.languages(), [locale("en"), locale("de")]);
}

#[test]
fn refuses_an_unknown_field() {
    assert_eq!(
        message_with(
            "mode = \"server\"\n",
            "mode = \"server\"\ncolour = \"blue\"\n"
        ),
        "site.toml, line 5, field \"colour\": unknown field `colour`, expected one of `name`, `address`, `mount`, `mode`, `slugs`, `default_language`, `languages`, `seo`, `reader`, `media`, `mail`, `alerts`, `code`, `preview_link_days`, `preview_link_hours`"
    );
}

#[test]
fn refuses_an_address_that_is_not_a_url() {
    assert_eq!(
        message_with("\"https://www.example.com\"", "\"www.example.com\""),
        "site.toml, line 2, field \"address\": the address is not a full URL starting with https://"
    );
}

#[test]
fn refuses_a_language_that_is_not_a_language_tag() {
    assert_eq!(
        message_with("[\"en\", \"de\"]", "[\"en\", \"english\"]"),
        "site.toml, line 6, field \"languages\": the language must be a two- or three-letter code such as en or de"
    );
}

#[test]
fn reads_the_headless_mode() {
    let text = DEMO.replacen("\"server\"", "\"headless\"", 1);
    let config = Config::from_toml(Path::new("site.toml"), &text, &Environment::empty()).unwrap();
    assert_eq!(config.mode(), Mode::Headless);
    assert!(!config.mode().has_pages());
    assert!(Mode::Server.has_pages());
    assert!(Mode::Static.has_pages());
}

#[test]
fn refuses_an_unknown_mode() {
    assert_eq!(
        message_with("\"server\"", "\"hybrid\""),
        "site.toml, line 4, field \"mode\": unknown variant `hybrid`, expected one of `server`, `static`, `headless`"
    );
}

#[test]
fn refuses_an_unknown_slug_mode() {
    assert_eq!(
        message_with(
            "mode = \"server\"\n",
            "mode = \"server\"\nslugs = \"latin\"\n"
        ),
        "site.toml, line 5, field \"slugs\": unknown variant `latin`, expected `ascii` or `unicode`"
    );
}

#[test]
fn refuses_a_mount_path_without_slashes() {
    assert_eq!(
        message_with("\"/blog/\"", "\"blog\""),
        "site.toml, line 3, field \"mount\": the mount path must start and end with /, like /blog/"
    );
}

#[test]
fn refuses_a_site_with_no_languages() {
    assert_eq!(
        message_with("[\"en\", \"de\"]", "[]"),
        "site.toml, line 6, field \"languages\": the site needs at least one language"
    );
}

#[test]
fn refuses_a_default_language_the_site_does_not_publish() {
    assert_eq!(
        message_with("default_language = \"en\"", "default_language = \"fr\""),
        "site.toml, line 5, field \"default_language\": the default language must be one of languages"
    );
}

#[test]
fn refuses_a_missing_field() {
    assert_eq!(
        message_with("name = \"Field Notes on SEO\"\n", ""),
        "site.toml, line 1, field \"name\": missing field `name`"
    );
}

#[test]
fn refuses_text_that_is_not_toml() {
    assert_eq!(
        message_with("\"Field Notes on SEO\"", "\"Field Notes on SEO"),
        "site.toml, line 1, field \"name\": invalid basic string, expected `\"`"
    );
}

#[test]
fn refuses_a_site_folder_with_no_settings_file() {
    let error = Config::load(Path::new("/no/such/site"), &Environment::empty()).unwrap_err();
    assert_eq!(error.to_string(), "could not read /no/such/site/site.toml");
    assert!(matches!(error, ConfigError::Read { .. }));
}

#[test]
fn the_environment_overrides_the_address() {
    let env = Environment::from_pairs(&[("KHALID_ADDRESS", "https://blog.example.org")]);
    let config = Config::from_toml(Path::new("site.toml"), DEMO, &env).unwrap();
    assert_eq!(config.address().as_str(), "https://blog.example.org/");
}

#[test]
fn the_environment_carries_an_agents_key_as_a_secret() {
    let made_for_this_test = "ki_0123456789abcdef_generated";
    let env = Environment::from_pairs(&[(Environment::KEY, made_for_this_test)]);
    let key = env.key().unwrap();
    assert_eq!(key.expose(), made_for_this_test);
    assert!(!format!("{env:?}").contains(made_for_this_test), "{env:?}");
    assert!(format!("{env:?}").contains("KHALID_KEY"), "{env:?}");
}

#[test]
fn an_empty_key_in_the_environment_is_no_key() {
    assert!(Environment::empty().key().is_none());
    let env = Environment::from_pairs(&[(Environment::KEY, "  ")]);
    assert!(env.key().is_none());
}

#[test]
fn refuses_an_address_in_the_environment_that_is_not_a_url() {
    let env = Environment::from_pairs(&[("KHALID_ADDRESS", "blog.example.org")]);
    let error = Config::from_toml(Path::new("site.toml"), DEMO, &env).unwrap_err();
    assert_eq!(
        error.to_string(),
        "the environment variable KHALID_ADDRESS is not a valid address"
    );
}
