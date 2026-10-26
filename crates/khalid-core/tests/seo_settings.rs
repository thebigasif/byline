//! The `[seo]` table of `site.toml`: which archive pages search engines may
//! list (KH-SEO-9).

use std::path::Path;

use khalid_core::{ArchiveIndex, Config, Environment, Indexing};

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
fn refuses_an_archive_type_it_does_not_know() {
    assert_eq!(
        read("\n[seo.index]\nmonths = \"noindex\"\n").unwrap_err(),
        "site.toml, line 8, field \"months\": unknown field `months`, expected one of `tags`, `categories`, `authors`, `dates`, `search`, `thin_archives_below`"
    );
}
