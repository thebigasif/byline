//! hreflang links from translation groups (task 1.13, KH-CON-8).

#![cfg(test)]

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, locale, path, url};
use khalid_core::{Indexing, RobotsDirectives};
use khalid_seo::{
    PageKind, SeoError, SeoPage, SeoSite, TranslationKey, head_tags, link_translations,
};

fn key(text: &str) -> Option<TranslationKey> {
    TranslationKey::try_new(text).ok()
}

fn hreflang_lines(site: &SeoSite, page: &SeoPage) -> Vec<String> {
    head_tags(site, page)
        .unwrap()
        .to_html()
        .lines()
        .filter(|line| line.contains("hreflang") || line.contains("og:locale"))
        .map(str::to_owned)
        .collect()
}

fn the_pair() -> Vec<SeoPage> {
    let mut en = internal_links_post();
    en.translation = key("internal-links");
    let mut de = internal_links_post_de();
    de.translation = key("internal-links");
    vec![de, en]
}

#[test]
fn a_translated_post_links_every_version_and_x_default() {
    let site = demo_site();
    let mut pages = the_pair();
    link_translations(&site, &mut pages).unwrap();
    insta::assert_snapshot!(hreflang_lines(&site, &pages[1]).join("\n"));
}

#[test]
fn every_version_in_a_group_carries_the_same_complete_set() {
    let site = demo_site();
    let mut pages = the_pair();
    link_translations(&site, &mut pages).unwrap();
    let de: Vec<String> = hreflang_lines(&site, &pages[0])
        .into_iter()
        .filter(|line| line.contains("hreflang"))
        .collect();
    let en: Vec<String> = hreflang_lines(&site, &pages[1])
        .into_iter()
        .filter(|line| line.contains("hreflang"))
        .collect();
    assert_eq!(de, en);
    assert_eq!(de.len(), 3, "two languages and x-default: {de:?}");
}

#[test]
fn without_the_default_language_x_default_is_the_first_site_language_present() {
    let mut site = demo_site();
    site.languages = vec![locale("en"), locale("fr"), locale("de")];
    let mut fr = internal_links_post_de();
    fr.language = locale("fr");
    fr.path = path("/liens-internes/");
    fr.translation = key("internal-links");
    let mut de = internal_links_post_de();
    de.translation = key("internal-links");
    let mut pages = vec![de, fr];
    link_translations(&site, &mut pages).unwrap();
    let lines = hreflang_lines(&site, &pages[0]).join("\n");
    assert!(
        lines.contains(
            r#"<link rel="alternate" hreflang="x-default" href="https://www.example.com/blog/fr/liens-internes/">"#
        ),
        "{lines}"
    );
}

#[test]
fn a_post_with_no_translation_still_names_itself_and_x_default() {
    let site = demo_site();
    let mut pages = vec![internal_links_post()];
    link_translations(&site, &mut pages).unwrap();
    let lines = hreflang_lines(&site, &pages[0]);
    let hreflang: Vec<&String> = lines.iter().filter(|l| l.contains("hreflang")).collect();
    assert_eq!(hreflang.len(), 2, "{lines:?}");
}

#[test]
fn a_site_in_one_language_writes_no_hreflang() {
    let mut site = demo_site();
    site.languages = vec![locale("en")];
    let mut pages = vec![internal_links_post()];
    link_translations(&site, &mut pages).unwrap();
    assert!(
        hreflang_lines(&site, &pages[0])
            .iter()
            .all(|line| !line.contains("hreflang"))
    );
}

#[test]
fn two_versions_in_one_language_are_refused() {
    let mut other = internal_links_post();
    other.path = path("/internal-links-again/");
    other.translation = key("internal-links");
    let mut pages = the_pair();
    pages.push(other);
    let error = link_translations(&demo_site(), &mut pages).unwrap_err();
    assert!(
        matches!(&error, SeoError::TranslationLanguage { key, language }
            if key == "internal-links" && language == "en"),
        "{error:?}"
    );
}

#[test]
fn a_noindex_or_canonicalised_version_is_left_out_of_the_group() {
    let site = demo_site();
    let mut pages = the_pair();
    let mut robots = RobotsDirectives::default();
    robots.indexing = Indexing::NoIndex;
    pages[0].robots = Some(robots);
    let mut syndicated = SeoPage::new(
        PageKind::Post,
        locale("fr"),
        path("/liens-internes/"),
        "Liens internes",
    );
    syndicated.translation = key("internal-links");
    syndicated.canonical = Some(url("https://serpdad.com/fr/liens-internes/"));
    pages.push(syndicated);
    link_translations(&site, &mut pages).unwrap();
    let english = hreflang_lines(&site, &pages[1]).join("\n");
    assert!(!english.contains("/de/"), "{english}");
    assert!(!english.contains("/fr/"), "{english}");
    assert_eq!(pages[0].alternates, []);
    assert_eq!(pages[2].alternates, []);
}

#[test]
fn a_translation_key_must_have_text() {
    assert!(TranslationKey::try_new("  ").is_err());
    assert_eq!(
        TranslationKey::try_new("internal-links").unwrap().as_str(),
        "internal-links"
    );
}
