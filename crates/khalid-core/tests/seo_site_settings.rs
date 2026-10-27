//! The site-wide SEO tables of `site.toml` (audit G11): titles, social,
//! verification, crawlers, feeds, links, publisher, locations and schema
//! templates. Core reads their shape; `khalid` checks each value with the
//! rules in `khalid-seo` before any command runs.

use std::path::Path;

use khalid_core::{
    Config, Environment, FeedContentSetting, LinkRelSetting, LinkTargetSetting, PublisherType,
    SeoSettings, WeekdaySetting,
};

const SITE: &str = r#"name = "Field Notes on SEO"
address = "https://www.example.com"
mount = "/blog/"
default_language = "en"
languages = ["en"]
"#;

const EVERY_TABLE: &str = r#"
[seo.titles]
separator = "·"
tagline = "Plain notes on search"

[seo.titles.post]
title = "{title} {sep} {site}"
description = "{excerpt}"

[seo.titles.category]
title = "{title} guides {sep} {site}"

[seo.social]
twitter = "@serpdad"

[seo.social.image]
url = "https://www.example.com/media/share.png"
alt = "The Field Notes logo on a white card"

[seo.verification]
google = "aBc123-_"
pinterest = "f00d"

[seo.verification.files]
"google1a2b3c4d.html" = "google-site-verification: google1a2b3c4d.html"

[seo.crawlers]
disallow = ["/drafts/"]
allow = ["Googlebot"]
block = ["GPTBot", "CCBot"]

[seo.feeds]
content = "excerpt"
after_item = "Thanks for reading."
source_line = "{title} first appeared on {site}."
limit = 50

[seo.links]
external = ["nofollow"]
target = "new-tab"
affiliate_domains = ["amazon.com"]
internal_domains = ["shop.example.com"]

[seo.publisher]
type = "person"
name = "Asif Khan"
same_as = ["https://www.linkedin.com/in/asif"]
founded = "2019-04-01"

[seo.publisher.logo]
url = "https://www.example.com/media/asif.png"
alt = "Asif, smiling"
width = 400
height = 400

[[seo.locations]]
type = "Dentist"
name = "Mill Lane Dental"
street = "12 Mill Lane"
locality = "Leeds"
postal_code = "LS1 4AB"
country = "GB"

[[seo.locations.hours]]
days = ["mon", "tue", "wed"]
opens = "09:00"
closes = "17:30"

[[seo.schema]]
category = "Courses"
type = "Course"

[seo.schema.properties]
provider = "{site}"
"#;

fn read(seo: &str) -> Result<SeoSettings, String> {
    let text = format!("{SITE}{seo}");
    Config::from_toml(Path::new("site.toml"), &text, &Environment::empty())
        .map(|config| config.seo().clone())
        .map_err(|error| error.to_string())
}

#[test]
fn a_site_without_the_tables_sets_nothing() {
    let seo = read("").unwrap();
    assert_eq!(seo, SeoSettings::default());
    assert_eq!(seo.titles.separator, None);
    assert_eq!(seo.titles.tagline, "");
    assert_eq!(seo.titles.post, None);
    assert_eq!(seo.social.twitter, None);
    assert_eq!(seo.social.image, None);
    assert_eq!(seo.feeds.content, FeedContentSetting::Full);
    assert_eq!(seo.feeds.limit, 20);
    assert_eq!(seo.links.target, LinkTargetSetting::SameTab);
    assert_eq!(seo.publisher.kind, PublisherType::Organization);
    assert_eq!(seo.publisher.name, None);
}

#[test]
fn reads_titles_social_and_verification() {
    let seo = read(EVERY_TABLE).unwrap();
    assert_eq!(seo.titles.separator.as_deref(), Some("·"));
    assert_eq!(seo.titles.tagline, "Plain notes on search");
    let post = seo.titles.post.as_ref().unwrap();
    assert_eq!(post.title.as_deref(), Some("{title} {sep} {site}"));
    assert_eq!(post.description.as_deref(), Some("{excerpt}"));
    let category = seo.titles.category.as_ref().unwrap();
    assert_eq!(category.description, None);
    assert_eq!(seo.social.twitter.as_deref(), Some("@serpdad"));
    let image = seo.social.image.as_ref().unwrap();
    assert_eq!(
        image.url.as_str(),
        "https://www.example.com/media/share.png"
    );
    assert_eq!((image.width, image.height), (1200, 630));
    assert_eq!(seo.verification.google.as_deref(), Some("aBc123-_"));
    assert_eq!(seo.verification.bing, None);
    let files: Vec<&str> = seo.verification.files.keys().map(String::as_str).collect();
    assert_eq!(files, ["google1a2b3c4d.html"]);
}

#[test]
fn reads_crawlers_feeds_and_links() {
    let seo = read(EVERY_TABLE).unwrap();
    assert_eq!(seo.crawlers.disallow, ["/drafts/"]);
    assert_eq!(seo.crawlers.allow, ["Googlebot"]);
    assert_eq!(seo.crawlers.block, ["GPTBot", "CCBot"]);
    assert_eq!(seo.feeds.content, FeedContentSetting::Excerpt);
    assert_eq!(seo.feeds.before_item, "");
    assert_eq!(seo.feeds.after_item, "Thanks for reading.");
    assert_eq!(
        seo.feeds.source_line.as_deref(),
        Some("{title} first appeared on {site}.")
    );
    assert_eq!(seo.feeds.limit, 50);
    assert_eq!(seo.links.external, [LinkRelSetting::NoFollow]);
    assert_eq!(seo.links.target, LinkTargetSetting::NewTab);
    assert_eq!(seo.links.affiliate_domains, ["amazon.com"]);
    assert_eq!(seo.links.internal_domains, ["shop.example.com"]);
}

#[test]
fn reads_the_publisher_its_locations_and_schema_templates() {
    let seo = read(EVERY_TABLE).unwrap();
    assert_eq!(seo.publisher.kind, PublisherType::Person);
    assert_eq!(seo.publisher.name.as_deref(), Some("Asif Khan"));
    assert_eq!(seo.publisher.founded.as_deref(), Some("2019-04-01"));
    let logo = seo.publisher.logo.as_ref().unwrap();
    assert_eq!((logo.width, logo.height), (400, 400));
    let [location] = seo.locations.as_slice() else {
        panic!("one location: {:?}", seo.locations);
    };
    assert_eq!(location.kind, "Dentist");
    assert_eq!(location.postal_code, "LS1 4AB");
    assert_eq!(location.region, None);
    let [hours] = location.hours.as_slice() else {
        panic!("one set of hours: {:?}", location.hours);
    };
    let days = [
        WeekdaySetting::Mon,
        WeekdaySetting::Tue,
        WeekdaySetting::Wed,
    ];
    assert_eq!(hours.days, days);
    assert_eq!(
        (hours.opens.as_str(), hours.closes.as_str()),
        ("09:00", "17:30")
    );
    let [template] = seo.schema.as_slice() else {
        panic!("one schema template: {:?}", seo.schema);
    };
    assert_eq!(template.category.as_deref(), Some("Courses"));
    assert_eq!(template.collection, None);
    assert_eq!(template.kind, "Course");
    assert_eq!(
        template.properties.get("provider").map(String::as_str),
        Some("{site}")
    );
}

#[test]
fn a_location_with_no_type_is_a_local_business() {
    let seo = read(
        "\n[[seo.locations]]\nname = \"Mill Lane\"\nstreet = \"12 Mill Lane\"\nlocality = \"Leeds\"\npostal_code = \"LS1\"\ncountry = \"GB\"\n",
    )
    .unwrap();
    assert_eq!(
        seo.locations.first().map(|l| l.kind.as_str()),
        Some("LocalBusiness")
    );
}

#[test]
fn refuses_a_page_type_the_titles_do_not_know() {
    let error = read("\n[seo.titles.posts]\ntitle = \"{title}\"\n").unwrap_err();
    assert!(
        error.starts_with("site.toml, line 7, field \"posts\": unknown field `posts`, expected one of `separator`, `tagline`, `home`, `post`"),
        "{error}"
    );
}

#[test]
fn refuses_a_link_rel_it_does_not_know() {
    assert_eq!(
        read("\n[seo.links]\nexternal = [\"noreferrer\"]\n").unwrap_err(),
        "site.toml, line 8, field \"external\": unknown variant `noreferrer`, expected one of `nofollow`, `sponsored`, `ugc`"
    );
}

#[test]
fn refuses_a_day_it_does_not_know() {
    let error = read(
        "\n[[seo.locations]]\nname = \"a\"\nstreet = \"b\"\nlocality = \"c\"\npostal_code = \"d\"\ncountry = \"GB\"\n[[seo.locations.hours]]\ndays = [\"monday\"]\nopens = \"09:00\"\ncloses = \"17:00\"\n",
    )
    .unwrap_err();
    assert!(error.contains("unknown variant `monday`"), "{error}");
}
