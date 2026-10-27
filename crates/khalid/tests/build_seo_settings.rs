//! `khalid build` applies the site-wide SEO tables of `site.toml` to every
//! page (audit G11): title templates, the share image, the Twitter account
//! and verification codes (KH-SEO-1, KH-SEO-2, KH-SEO-7).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = at(into, &entry.file_name().to_string_lossy());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Titles, social and verification, as a site owner writes them.
const HEAD_TABLES: &str = r#"
[seo.titles]
separator = "·"
tagline = "Plain notes on search"

[seo.titles.post]
title = "{title} {sep} {site} {sep} {author}"

[seo.social]
twitter = "serpdad"

[seo.social.image]
url = "https://www.example.com/media/share.png"
alt = "The Field Notes logo on a white card"

[seo.verification]
google = "aBc123-_"
bing = "F00D42"
"#;

/// The demo site with `settings` added to the end of its `site.toml`,
/// built into a new folder. Returns the site and the output folder.
#[cfg(test)]
fn built_with(settings: &str) -> (TempDir, TempDir) {
    let site = tempfile::tempdir().unwrap();
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    copy_dir(&demo, site.path());
    let toml = at(site.path(), "site.toml");
    let text = fs::read_to_string(&toml).unwrap();
    fs::write(&toml, format!("{text}{settings}")).unwrap();
    let out = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.path().to_str().unwrap(), "build", "--out"])
        .arg(out.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (site, out)
}

/// The tags of the page at `rel` that the head settings change: the title,
/// the description, verification, Open Graph and Twitter.
#[cfg(test)]
fn head_lines(out: &TempDir, rel: &str) -> String {
    let page = fs::read_to_string(at(out.path(), rel)).unwrap();
    page.lines()
        .filter(|line| {
            line.starts_with("<title>")
                || line.starts_with("<meta name=\"description\"")
                || line.starts_with("<meta name=\"google")
                || line.starts_with("<meta name=\"msvalidate")
                || line.starts_with("<meta property=\"og:")
                || line.starts_with("<meta name=\"twitter:")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_head_tables_reach_the_home_page() {
    let (_site, out) = built_with(HEAD_TABLES);
    insta::assert_snapshot!(head_lines(&out, "index.html"));
}

/// Crawlers and feeds, as a site owner writes them.
const FILE_TABLES: &str = r#"
[seo.crawlers]
disallow = ["/drafts/"]
allow = ["Googlebot"]
block = ["GPTBot", "CCBot"]

[seo.feeds]
content = "excerpt"
source_line = "{title} first appeared on {site}."
limit = 2

[seo.links]
external = ["nofollow"]
affiliate_domains = ["amazon.com"]
"#;

#[test]
fn the_crawler_table_shapes_robots_txt() {
    let (_site, out) = built_with(FILE_TABLES);
    insta::assert_snapshot!(fs::read_to_string(at(out.path(), "robots.txt")).unwrap());
}

#[test]
fn the_feed_table_shapes_every_feed() {
    let (_site, out) = built_with(FILE_TABLES);
    let rss = fs::read_to_string(at(out.path(), "feed.xml")).unwrap();
    assert_eq!(rss.matches("<item>").count(), 2, "{rss}");
    assert!(rss.contains("first appeared on"), "{rss}");
    let json = fs::read_to_string(at(out.path(), "de/feed.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let items = value["items"].as_array().unwrap();
    assert!(items.len() <= 2, "{json}");
}

/// The publisher, a location and a schema template, as a site owner writes them.
const GRAPH_TABLES: &str = r#"
[seo.publisher]
type = "person"
name = "Asif Khan"
same_as = ["https://www.linkedin.com/in/asif"]

[[seo.locations]]
type = "Dentist"
name = "Mill Lane Dental"
street = "12 Mill Lane"
locality = "Leeds"
postal_code = "LS1 4AB"
country = "GB"

[[seo.locations.hours]]
days = ["mon", "tue"]
opens = "09:00"
closes = "17:30"

[[seo.schema]]
collection = "posts"
type = "Course"

[seo.schema.properties]
provider = "{site}"
"#;

/// The JSON-LD graph of the page at `rel`, one node to a line.
#[cfg(test)]
fn graph(out: &TempDir, rel: &str) -> String {
    let page = fs::read_to_string(at(out.path(), rel)).unwrap();
    let (_, rest) = page.split_once("application/ld+json\">").unwrap();
    let (json, _) = rest.split_once("</script>").unwrap();
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    value["@graph"]
        .as_array()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_publisher_and_its_locations_reach_the_home_page_graph() {
    let (_site, out) = built_with(GRAPH_TABLES);
    insta::assert_snapshot!(graph(&out, "index.html"));
}

#[test]
fn a_post_in_a_templated_collection_carries_the_template_node() {
    let (_site, out) = built_with(GRAPH_TABLES);
    let nodes = graph(&out, "why-internal-links-matter/index.html");
    assert!(nodes.contains(r#""@type":"Course""#), "{nodes}");
    assert!(
        nodes.contains(r#""provider":"Field Notes on SEO""#),
        "{nodes}"
    );
}

#[test]
fn a_post_title_follows_its_template() {
    let (_site, out) = built_with(HEAD_TABLES);
    let tags = head_lines(&out, "why-internal-links-matter/index.html");
    let title = "<title>Why internal links matter · Field Notes on SEO · Asif</title>";
    assert!(tags.contains(title), "{tags}");
    assert!(
        tags.contains(r#"<meta name="twitter:site" content="@serpdad">"#),
        "{tags}"
    );
    assert!(!tags.contains("google-site-verification"), "{tags}");
}
