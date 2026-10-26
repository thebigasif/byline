//! Addresses built from permalink patterns, the mount path and the language
//! prefix, and aliases that become redirects: the done line of task 1.9.

use std::path::Path;

use chrono::NaiveDate;
use khalid_core::{Config, Environment, Locale, Slug};
use khalid_routes::{AddressError, PostAddress, Routes};

/// Settings for a site mounted at `mount`, in English and German.
#[cfg(test)]
fn site(mount: &str) -> Config {
    let text = format!(
        "name = \"Field Notes on SEO\"\naddress = \"https://www.example.com\"\n\
         mount = \"{mount}\"\ndefault_language = \"en\"\nlanguages = [\"en\", \"de\"]\n"
    );
    Config::from_toml(Path::new("site.toml"), &text, &Environment::empty()).unwrap()
}

#[cfg(test)]
fn routes() -> Routes {
    Routes::from_toml(
        Path::new("routes.toml"),
        "[collections.news]\npermalink = \"/news/{year}/{slug}/\"\n\n\
         [collections.changelog]\npermalink = \"/{collection}/{year}/{month}/{day}/{slug}/\"\n",
    )
    .unwrap()
}

/// The address of a post in `collection` and `language`, dated 2026-09-04.
#[cfg(test)]
fn address(routes: &Routes, site: &Config, collection: &str, language: &str) -> String {
    let locale = Locale::try_new(language).unwrap();
    let slug = Slug::try_new("internal-links").unwrap();
    let date = NaiveDate::from_ymd_opt(2026, 9, 4);
    routes
        .address(site, &PostAddress::new(collection, &locale, &slug, date))
        .unwrap()
}

#[test]
fn the_default_post_address_sits_under_the_mount_path() {
    let routes = Routes::default();
    assert_eq!(
        address(&routes, &site("/blog/"), "posts", "en"),
        "/blog/internal-links/"
    );
}

#[test]
fn a_dated_collection_fills_in_the_year() {
    assert_eq!(
        address(&routes(), &site("/blog/"), "news", "en"),
        "/blog/news/2026/internal-links/"
    );
}

#[test]
fn every_placeholder_is_filled_with_two_digit_months_and_days() {
    assert_eq!(
        address(&routes(), &site("/"), "changelog", "en"),
        "/changelog/2026/09/04/internal-links/"
    );
}

#[test]
fn other_languages_get_their_prefix_and_the_default_language_does_not() {
    let site = site("/blog/");
    assert_eq!(
        address(&routes(), &site, "news", "de"),
        "/blog/de/news/2026/internal-links/"
    );
    assert_eq!(
        address(&routes(), &site, "posts", "de"),
        "/blog/de/internal-links/"
    );
    assert_eq!(
        address(&routes(), &site, "posts", "en"),
        "/blog/internal-links/"
    );
}

#[test]
fn the_mount_path_moves_every_address() {
    assert_eq!(
        address(&routes(), &site("/"), "posts", "en"),
        "/internal-links/"
    );
    assert_eq!(
        address(&routes(), &site("/resources/blog/"), "news", "de"),
        "/resources/blog/de/news/2026/internal-links/"
    );
}

#[test]
fn a_collection_without_a_route_sits_under_its_name() {
    assert_eq!(
        address(&Routes::default(), &site("/blog/"), "glossary", "en"),
        "/blog/glossary/internal-links/"
    );
}

#[test]
fn a_dated_pattern_needs_a_date() {
    let locale = Locale::try_new("en").unwrap();
    let slug = Slug::try_new("internal-links").unwrap();
    let error = routes()
        .address(
            &site("/blog/"),
            &PostAddress::new("news", &locale, &slug, None),
        )
        .unwrap_err();
    assert!(matches!(error, AddressError::NoDate { .. }), "{error:?}");
}

#[test]
fn an_alias_becomes_a_redirect_to_the_post() {
    let site = site("/blog/");
    let routes = routes();
    let locale = Locale::try_new("en").unwrap();
    let slug = Slug::try_new("internal-links").unwrap();
    let post = PostAddress::new("news", &locale, &slug, NaiveDate::from_ymd_opt(2026, 9, 4));
    let redirect = routes
        .redirect(&site, "/blog/2019/05/old-internal-links/", &post)
        .unwrap();
    assert_eq!(redirect.from(), "/blog/2019/05/old-internal-links/");
    assert_eq!(redirect.to(), "/blog/news/2026/internal-links/");
}

#[test]
fn an_alias_outside_the_mount_path_is_refused() {
    let site = site("/blog/");
    let locale = Locale::try_new("en").unwrap();
    let slug = Slug::try_new("internal-links").unwrap();
    let post = PostAddress::new("posts", &locale, &slug, None);
    for (alias, expected) in [
        ("/2019/old-post/", "outside"),
        ("blog/old-post/", "path"),
        ("/blog/../admin/", "path"),
        ("/blog/old post/", "path"),
    ] {
        let error = Routes::default().redirect(&site, alias, &post).unwrap_err();
        let said = error.to_string();
        assert!(said.contains(expected), "{alias}: {said}");
    }
}

#[test]
fn an_alias_that_is_the_post_itself_is_refused() {
    let site = site("/blog/");
    let locale = Locale::try_new("en").unwrap();
    let slug = Slug::try_new("internal-links").unwrap();
    let post = PostAddress::new("posts", &locale, &slug, None);
    let error = Routes::default()
        .redirect(&site, "/blog/internal-links/", &post)
        .unwrap_err();
    assert!(
        matches!(error, AddressError::SameAddress { .. }),
        "{error:?}"
    );
}
