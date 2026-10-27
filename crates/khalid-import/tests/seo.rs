//! The SEO plugins' fields, read from a post's custom fields: one fixture
//! per plugin under `fixtures/wordpress/seo/`.

use std::fs;

use khalid_core::{AbsoluteUrl, RobotsDirectives};
use khalid_import::wordpress::{SeoMeta, SeoPlugin, read_wxr};

#[cfg(test)]
fn fixture(plugin: &str) -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    fs::read_to_string(format!("{dir}/fixtures/wordpress/seo/{plugin}.xml")).unwrap()
}

#[cfg(test)]
fn seo_in(text: &str, id: u64) -> SeoMeta {
    let export = read_wxr(text).unwrap();
    SeoMeta::read(export.item(id).unwrap(), &export.site)
}

#[cfg(test)]
fn seo(plugin: &str, id: u64) -> SeoMeta {
    seo_in(&fixture(plugin), id)
}

#[cfg(test)]
fn robots(words: &str) -> RobotsDirectives {
    words.parse().unwrap()
}

#[cfg(test)]
fn unread(seo: &SeoMeta) -> Vec<(SeoPlugin, &str, &str)> {
    seo.unread
        .iter()
        .map(|field| (field.plugin, field.key.as_str(), field.value.as_str()))
        .collect()
}

#[test]
fn yoast_title_description_canonical_robots_and_focus() {
    let seo = seo("yoast", 20);
    assert_eq!(
        seo.title.as_deref(),
        Some("Why rye bread needs time - Northfield Bakery Blog"),
        "variables filled in, the page number dropped"
    );
    assert_eq!(
        seo.description.as_deref(),
        Some("Rye ferments slowly. Here is how long to wait, and why it pays off.")
    );
    assert_eq!(
        seo.canonical.as_ref().map(AbsoluteUrl::as_str),
        Some("https://northfield.example.com/rye-guide/")
    );
    assert_eq!(
        seo.robots,
        Some(robots("noindex, nofollow, noarchive, nosnippet"))
    );
    assert_eq!(seo.focus_keywords, ["rye bread"]);
    assert_eq!(seo.primary_category, Some(4));
    assert!(seo.unread.is_empty(), "{:?}", seo.unread);
}

#[test]
fn yoast_fields_that_cannot_be_carried_are_listed() {
    let seo = seo("yoast", 21);
    assert_eq!(seo.title, None, "a variable Khalid cannot fill");
    assert_eq!(
        seo.description.as_deref(),
        Some("From a soft white loaf to a dark rye.")
    );
    assert_eq!(seo.canonical, None);
    assert_eq!(seo.robots, None, "index and follow are the defaults");
    assert_eq!(
        unread(&seo),
        [
            (
                SeoPlugin::Yoast,
                "_yoast_wpseo_title",
                "Ten loaves for %%currentyear%%"
            ),
            (SeoPlugin::Yoast, "_yoast_wpseo_canonical", "rye-guide"),
        ]
    );
}

#[test]
fn a_variable_that_fills_with_nothing_leaves_the_field_empty() {
    let text = fixture("yoast").replace("From a soft white loaf to a dark rye.", "");
    let seo = seo_in(&text, 21);
    assert_eq!(seo.description, None);
    assert_eq!(seo.unread.len(), 2, "only the title and canonical");
}

#[test]
fn rank_math_fields_with_its_serialised_robots() {
    let seo = seo("rankmath", 30);
    assert_eq!(
        seo.title.as_deref(),
        Some("Feeding a sourdough starter - Northfield Bakery Blog")
    );
    assert_eq!(
        seo.description.as_deref(),
        Some("How much flour and water a starter needs, and how often.")
    );
    assert_eq!(
        seo.canonical.as_ref().map(AbsoluteUrl::as_str),
        Some("https://northfield.example.com/starters/")
    );
    let words = "noindex, nofollow, noimageindex, max-snippet:160, \
                 max-video-preview:-1, max-image-preview:large";
    assert_eq!(seo.robots, Some(robots(words)));
    assert_eq!(
        seo.focus_keywords,
        ["sourdough starter", "levain", "feeding schedule"]
    );
    assert_eq!(seo.primary_category, Some(3));
    assert!(seo.unread.is_empty(), "{:?}", seo.unread);
}

#[test]
fn rank_math_comes_before_the_yoast_fields_it_imported() {
    let seo = seo("rankmath", 31);
    assert_eq!(
        seo.title.as_deref(),
        Some("More rise from the same flour - Northfield")
    );
    assert_eq!(
        seo.description.as_deref(),
        Some("Get 50% more rise with a warmer proof."),
        "a lone % is text, not a variable"
    );
    assert_eq!(
        seo.focus_keywords,
        ["rise"],
        "Yoast fills what Rank Math left"
    );
    assert_eq!(seo.robots, None);
    assert_eq!(
        unread(&seo),
        [(
            SeoPlugin::RankMath,
            "rank_math_advanced_robots",
            r#"a:1:{s:11:"max-snippet";s:3:"160""#
        )]
    );
}
