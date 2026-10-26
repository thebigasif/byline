//! A site's wikilinks resolve to the addresses `khalid-routes` gives each
//! post, page and glossary term, in the same language (KH-CON-6).

use std::path::Path;

use khalid_content::{ContentTree, SiteContent, Wikilinks, read_content, render_linked};
use khalid_core::{Config, Environment, Locale};
use khalid_routes::Routes;

const DEMO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/demo-site");

#[cfg(test)]
fn demo() -> (Config, SiteContent) {
    let site = Path::new(DEMO);
    let config = Config::load(site, &Environment::empty()).unwrap();
    let content = read_content(&ContentTree::walk(site).unwrap()).unwrap();
    (config, content)
}

#[cfg(test)]
fn html_of(config: &Config, content: &SiteContent, locale: &Locale, key: &str) -> String {
    let links = Wikilinks::for_language(content, config, &Routes::default(), locale);
    let doc = content
        .posts
        .iter()
        .find(|doc| &doc.locale == locale && doc.front.translation.as_deref() == Some(key))
        .unwrap();
    render_linked(&doc.body, &doc.locale, config.slugs(), &links)
        .unwrap()
        .html
}

#[test]
fn the_demo_glossary_links_resolve_in_english() {
    let (config, content) = demo();
    let en = Locale::try_new("en").unwrap();
    let links = Wikilinks::for_language(&content, &config, &Routes::default(), &en);
    let link = "<a href=\"/blog/glossary/topical-authority/\">Topical authority</a>";
    for key in ["internal-links", "topic-cluster"] {
        let html = html_of(&config, &content, &en, key);
        assert!(html.contains(link), "{key}: {html}");
    }
    let term = links.get("planning-a-topic-cluster").unwrap();
    assert_eq!(term.address(), "/blog/planning-a-topic-cluster/");
    assert_eq!(term.title(), "Planning a topic cluster in an afternoon");
}

#[test]
fn the_demo_glossary_links_resolve_in_german() {
    let (config, content) = demo();
    let de = Locale::try_new("de").unwrap();
    let links = Wikilinks::for_language(&content, &config, &Routes::default(), &de);
    let link = "<a href=\"/blog/de/glossary/thematische-autoritaet/\">Thematische Autorität</a>";
    for key in ["internal-links", "topic-cluster"] {
        let html = html_of(&config, &content, &de, key);
        assert!(html.contains(link), "{key}: {html}");
    }
    assert_eq!(links.get("topical-authority"), None, "no English targets");
}

#[test]
fn pages_live_at_the_root_of_their_language() {
    let (config, content) = demo();
    let de = Locale::try_new("de").unwrap();
    let links = Wikilinks::for_language(&content, &config, &Routes::default(), &de);
    let page = content.pages.iter().find(|doc| doc.locale == de).unwrap();
    let slug = page.front.slug.as_ref().unwrap();
    assert_eq!(
        links.get(slug.as_str()).unwrap().address(),
        format!("/blog/de/{slug}/")
    );
}

#[test]
fn a_collection_pattern_from_routes_toml_shapes_the_address() {
    let (config, content) = demo();
    let routes = Routes::from_toml(
        Path::new("routes.toml"),
        "[collections.posts]\npermalink = \"/{year}/{slug}/\"\n",
    )
    .unwrap();
    let en = Locale::try_new("en").unwrap();
    let links = Wikilinks::for_language(&content, &config, &routes, &en);
    assert_eq!(
        links.get("planning-a-topic-cluster").unwrap().address(),
        "/blog/2026/planning-a-topic-cluster/"
    );
}
