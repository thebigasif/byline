//! The `[nav]` and `[announcement]` tables of `site.toml`: menus in the
//! order written, one optional line above every page, and plain messages
//! for what a page could not show.

use std::path::Path;

use khalid_core::{Config, Environment, Locale, NavSettings, SiteHref};

const DEMO: &str = r#"name = "Field Notes on SEO"
address = "https://www.example.com"
mount = "/blog/"
default_language = "en"
languages = ["en", "de"]
"#;

#[cfg(test)]
fn read(tables: &str) -> Result<Config, String> {
    let text = format!("{DEMO}\n{tables}");
    Config::from_toml(Path::new("site.toml"), &text, &Environment::empty())
        .map_err(|error| error.to_string())
}

#[test]
fn a_site_without_the_tables_has_empty_menus_and_no_announcement() {
    let config = read("").unwrap();
    assert_eq!(config.nav(), &NavSettings::default());
    assert_eq!(config.announcement(), None);
}

#[test]
fn menus_keep_their_order_and_their_language() {
    let config = read(
        "[[nav.primary]]\nlabel = \"Guides\"\nhref = \"/blog/guides/\"\n\n\
         [[nav.primary]]\nlabel = \"Shop\"\nhref = \"https://shop.example.com/\"\n\n\
         [[nav.footer]]\nlabel = \"Impressum\"\nhref = \"/blog/de/impressum/\"\nlanguage = \"de\"\n",
    )
    .unwrap();
    let nav = config.nav();
    let primary: Vec<(&str, &str)> = nav
        .primary
        .iter()
        .map(|link| (link.label.as_str(), link.href.as_str()))
        .collect();
    assert_eq!(
        primary,
        [
            ("Guides", "/blog/guides/"),
            ("Shop", "https://shop.example.com/")
        ]
    );
    assert_eq!(nav.primary[0].language, None);
    assert_eq!(nav.footer[0].language, Some(Locale::try_new("de").unwrap()));
}

#[test]
fn the_announcement_has_its_text_and_an_optional_link() {
    let config = read(
        "[announcement]\ntext = \"Our technical SEO course opens on 3 November.\"\nhref = \"/blog/course/\"\n",
    )
    .unwrap();
    let bar = config.announcement().unwrap();
    assert_eq!(
        bar.text.as_str(),
        "Our technical SEO course opens on 3 November."
    );
    assert_eq!(
        bar.href.as_ref().map(SiteHref::as_str),
        Some("/blog/course/")
    );
    let plain = read("[announcement]\ntext = \"We moved.\"\n").unwrap();
    assert_eq!(plain.announcement().unwrap().href, None);
}

#[test]
fn refuses_a_link_that_is_not_a_path_or_a_full_address() {
    let message =
        read("[[nav.primary]]\nlabel = \"Run\"\nhref = \"javascript:alert(1)\"\n").unwrap_err();
    assert!(
        message.starts_with("site.toml, line 9, field \"href\": use a path on this site"),
        "{message}"
    );
}

#[test]
fn refuses_a_blank_label_and_a_long_announcement() {
    let blank = read("[[nav.footer]]\nlabel = \" \"\nhref = \"/blog/\"\n").unwrap_err();
    assert!(
        blank.contains("field \"label\": write the words a reader sees here"),
        "{blank}"
    );
    let long = format!("[announcement]\ntext = \"{}\"\n", "a".repeat(161));
    let long = read(&long).unwrap_err();
    assert!(long.contains("160 characters or fewer"), "{long}");
}

#[test]
fn refuses_a_language_the_site_does_not_have() {
    let message = read("[announcement]\ntext = \"Bonjour.\"\nlanguage = \"fr\"\n").unwrap_err();
    assert!(
        message.starts_with("site.toml, line 9, field \"language\": a menu link or the announcement names a language"),
        "{message}"
    );
}

#[test]
fn refuses_an_unknown_key_in_a_menu_link() {
    let message = read("[[nav.primary]]\nlabel = \"Guides\"\nhref = \"/blog/\"\nicon = \"star\"\n")
        .unwrap_err();
    assert!(message.contains("unknown field `icon`"), "{message}");
}
