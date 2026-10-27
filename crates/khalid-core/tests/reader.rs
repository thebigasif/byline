//! The `[reader]` table of `site.toml`: every reader feature on by default,
//! each one switched off by name, and the home page sections.

use std::path::Path;

use khalid_core::{Config, Environment, HomeSection, Reader, Slug};

const DEMO: &str = r#"name = "Field Notes on SEO"
address = "https://www.example.com"
mount = "/blog/"
default_language = "en"
languages = ["en", "de"]
"#;

#[cfg(test)]
fn read(reader: &str) -> Result<Config, String> {
    let text = format!("{DEMO}\n{reader}");
    Config::from_toml(Path::new("site.toml"), &text, &Environment::empty())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
fn slug(text: &str) -> Slug {
    Slug::try_new(text).unwrap()
}

#[test]
fn every_feature_is_on_without_a_reader_table() {
    let config = read("").unwrap();
    assert_eq!(config.reader(), &Reader::default());
    let reader = config.reader();
    for (name, on) in [
        ("byline", reader.byline),
        ("reviewed_by", reader.reviewed_by),
        ("reading_time", reader.reading_time),
        ("table_of_contents", reader.table_of_contents),
        ("heading_links", reader.heading_links),
        ("enlarge_images", reader.enlarge_images),
        ("author_box", reader.author_box),
        ("related_posts", reader.related_posts),
        ("rich_cards", reader.rich_cards),
        ("topic_links", reader.topic_links),
        ("authors_page", reader.authors_page),
        ("archive", reader.archive),
        ("search_shortcut", reader.search_shortcut),
    ] {
        assert!(on, "{name} is on by default");
    }
    assert!(reader.home_sections.is_empty(), "one newest-first list");
}

#[test]
fn a_feature_switches_off_by_name_and_the_rest_stay_on() {
    let config = read("[reader]\ntable_of_contents = false\narchive = false\n").unwrap();
    let reader = config.reader();
    assert!(!reader.table_of_contents);
    assert!(!reader.archive);
    assert!(reader.reading_time);
    assert!(reader.related_posts);
}

#[test]
fn home_sections_keep_their_order() {
    let config = read(
        "[[reader.home_sections]]\ncategory = \"technical-seo\"\n\n\
         [[reader.home_sections]]\ncollection = \"guides\"\n\n\
         [[reader.home_sections]]\nchannel = \"featured\"\n",
    )
    .unwrap();
    assert_eq!(
        config.reader().home_sections,
        [
            HomeSection::Category(slug("technical-seo")),
            HomeSection::Collection(slug("guides")),
            HomeSection::Channel(slug("featured")),
        ]
    );
}

#[test]
fn refuses_an_unknown_reader_feature() {
    let message = read("[reader]\ndark_mode = true\n").unwrap_err();
    assert!(
        message.starts_with(
            "site.toml, line 8, field \"dark_mode\": unknown field `dark_mode`, expected one of `byline`"
        ),
        "{message}"
    );
}

#[test]
fn refuses_a_home_section_of_an_unknown_kind() {
    let message = read("[[reader.home_sections]]\nauthor = \"lena-vogt\"\n").unwrap_err();
    assert!(
        message.contains(
            "unknown variant `author`, expected one of `category`, `collection`, `channel`"
        ),
        "{message}"
    );
}

#[test]
fn refuses_a_home_section_that_is_not_a_slug() {
    let message = read("[[reader.home_sections]]\ncategory = \"Technical SEO\"\n").unwrap_err();
    assert!(
        message.starts_with("site.toml, line 7") && message.contains("only a to z"),
        "{message}"
    );
}
