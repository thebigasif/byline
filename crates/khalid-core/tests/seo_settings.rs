//! The `[seo]` table of `site.toml`: which archive pages search engines may
//! list (KH-SEO-9), whether changed pages are sent to `IndexNow`
//! (KH-SEO-11), and what links to other sites get (KH-SEO-10).

use std::path::Path;

use khalid_core::{
    AbsoluteUrl, ArchiveIndex, Config, DomainName, Environment, ExternalRel, IndexNowKey,
    IndexNowSettings, Indexing, LinkOpening, LinkSettings,
};

const SITE: &str = r#"name = "Field Notes on SEO"
address = "https://www.example.com"
mount = "/blog/"
default_language = "en"
languages = ["en", "de"]
"#;

#[cfg(test)]
fn read(seo: &str) -> Result<Config, String> {
    let text = format!("{SITE}{seo}");
    Config::from_toml(Path::new("site.toml"), &text, &Environment::empty())
        .map_err(|error| error.to_string())
}

#[test]
fn a_site_without_the_table_gets_the_defaults() {
    let config = read("").unwrap();
    let index = &config.seo().index;
    assert_eq!(index, &ArchiveIndex::default());
    assert_eq!(index.tags, Indexing::Index);
    assert_eq!(index.categories, Indexing::Index);
    assert_eq!(index.authors, Indexing::Index);
    assert_eq!(index.dates, Indexing::NoIndex);
    assert_eq!(index.search, Indexing::NoIndex);
    assert_eq!(index.thin_archives_below, 3);
}

#[test]
fn reads_each_archive_setting() {
    let config = read(
        r#"
[seo.index]
tags = "noindex"
categories = "noindex"
authors = "noindex"
dates = "index"
search = "index"
thin_archives_below = 0
"#,
    )
    .unwrap();
    let index = &config.seo().index;
    assert_eq!(index.tags, Indexing::NoIndex);
    assert_eq!(index.categories, Indexing::NoIndex);
    assert_eq!(index.authors, Indexing::NoIndex);
    assert_eq!(index.dates, Indexing::Index);
    assert_eq!(index.search, Indexing::Index);
    assert_eq!(index.thin_archives_below, 0);
}

#[test]
fn a_setting_left_out_keeps_its_default() {
    let config = read("\n[seo.index]\ntags = \"noindex\"\n").unwrap();
    let index = &config.seo().index;
    assert_eq!(index.tags, Indexing::NoIndex);
    assert_eq!(index.dates, Indexing::NoIndex);
    assert_eq!(index.thin_archives_below, 3);
}

#[test]
fn refuses_a_value_that_is_not_index_or_noindex() {
    assert_eq!(
        read("\n[seo.index]\ntags = \"hidden\"\n").unwrap_err(),
        "site.toml, line 8, field \"tags\": unknown variant `hidden`, expected `index` or `noindex`"
    );
}

#[test]
fn indexnow_is_off_without_its_table() {
    let config = read("").unwrap();
    assert_eq!(config.seo().indexnow, None);
}

#[test]
fn the_indexnow_table_switches_it_on_with_the_shared_endpoint() {
    let config = read("\n[seo.indexnow]\nkey = \"3f9c0b5e2a8d4c71\"\n").unwrap();
    let indexnow = config.seo().indexnow.as_ref().unwrap();
    assert_eq!(indexnow.key.as_str(), "3f9c0b5e2a8d4c71");
    assert_eq!(indexnow.endpoint, None);
    assert_eq!(
        AbsoluteUrl::parse(IndexNowSettings::SHARED_ENDPOINT)
            .unwrap()
            .as_str(),
        "https://api.indexnow.org/indexnow"
    );
}

#[test]
fn reads_another_indexnow_endpoint() {
    let config = read(
        "\n[seo.indexnow]\nkey = \"3f9c0b5e2a8d4c71\"\nendpoint = \"https://www.bing.com/indexnow\"\n",
    )
    .unwrap();
    let indexnow = config.seo().indexnow.as_ref().unwrap();
    assert_eq!(
        indexnow.endpoint.as_ref().unwrap().as_str(),
        "https://www.bing.com/indexnow"
    );
}

#[test]
fn refuses_an_indexnow_key_with_a_character_search_engines_refuse() {
    assert_eq!(
        read("\n[seo.indexnow]\nkey = \"3f9c 0b5e\"\n").unwrap_err(),
        "site.toml, line 8, field \"key\": the IndexNow key holds a character that is not allowed, expected a key of 8 to 128 letters, digits and hyphens"
    );
}

#[test]
fn refuses_an_indexnow_table_without_a_key() {
    assert_eq!(
        read("\n[seo.indexnow]\nendpoint = \"https://www.bing.com/indexnow\"\n").unwrap_err(),
        "site.toml, line 7, field \"key\": missing field `key`"
    );
}

#[test]
fn indexnow_keys_are_eight_to_128_letters_digits_and_hyphens() {
    for good in ["abcdefgh", "3F9C-0b5e-2a8d-4c71", &"a".repeat(128)] {
        assert!(IndexNowKey::try_new(good).is_ok(), "{good:?}");
    }
    for bad in [
        "",
        "abcdefg",
        &"a".repeat(129),
        "abc_defgh",
        "abcdéfgh",
        "../../etc",
    ] {
        assert!(IndexNowKey::try_new(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn links_get_nothing_added_without_the_links_table() {
    let config = read("").unwrap();
    assert_eq!(config.seo().links, LinkSettings::default());
    assert_eq!(config.seo().links.open, LinkOpening::SameTab);
}

#[test]
fn reads_each_link_setting() {
    let config = read(
        r#"
[seo.links]
rel = ["nofollow", "ugc"]
open = "new-tab"
affiliate_domains = ["Amazon.com", "shareasale.com"]
internal_domains = ["shop.example.com"]
"#,
    )
    .unwrap();
    let links = &config.seo().links;
    assert_eq!(links.rel, [ExternalRel::NoFollow, ExternalRel::Ugc]);
    assert_eq!(links.open, LinkOpening::NewTab);
    let names: Vec<&str> = links
        .affiliate_domains
        .iter()
        .map(DomainName::as_str)
        .collect();
    assert_eq!(names, ["amazon.com", "shareasale.com"]);
    assert_eq!(links.internal_domains[0].as_str(), "shop.example.com");
}

#[test]
fn refuses_an_address_where_a_domain_name_belongs() {
    assert_eq!(
        read("\n[seo.links]\naffiliate_domains = [\"https://amazon.com/\"]\n").unwrap_err(),
        "site.toml, line 8, field \"affiliate_domains\": `https://amazon.com/` is not a domain name, expected a name such as amazon.com, with no https:// and no path"
    );
}

#[test]
fn domain_names_are_letters_digits_hyphens_and_inner_dots() {
    for good in ["amazon.com", "AMAZON.co.uk", "my-shop.example"] {
        assert!(DomainName::try_new(good).is_ok(), "{good:?}");
    }
    for bad in [
        "",
        ".amazon.com",
        "amazon.com.",
        "amazon.com/x",
        "amazon com",
    ] {
        assert!(DomainName::try_new(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn refuses_an_archive_type_it_does_not_know() {
    assert_eq!(
        read("\n[seo.index]\nmonths = \"noindex\"\n").unwrap_err(),
        "site.toml, line 8, field \"months\": unknown field `months`, expected one of `tags`, `categories`, `authors`, `dates`, `search`, `thin_archives_below`"
    );
}
