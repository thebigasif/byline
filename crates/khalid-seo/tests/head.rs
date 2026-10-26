//! The `<head>` tags for the demo site's page types (task 1.13).

mod common;

use common::{demo_site, internal_links_post, internal_links_post_de, locale, path, url};
use khalid_seo::{
    Following, Indexing, MetaTemplate, PageKind, RobotsDirectives, SeoPage, UrlPath, head_tags,
};

#[test]
fn a_post_gets_title_description_canonical_and_robots() {
    let tags = head_tags(&demo_site(), &internal_links_post()).unwrap();
    insta::assert_snapshot!(tags.to_html());
}

#[test]
fn the_home_page_uses_the_site_name_and_tagline() {
    let home = SeoPage::new(PageKind::Home, locale("en"), UrlPath::root(), "");
    let tags = head_tags(&demo_site(), &home).unwrap();
    insta::assert_snapshot!(tags.to_html());
}

#[test]
fn a_german_page_is_canonical_under_the_language_prefix() {
    let tags = head_tags(&demo_site(), &internal_links_post_de()).unwrap();
    let html = tags.to_html();
    assert!(
        html.contains(
            r#"<link rel="canonical" href="https://www.example.com/blog/de/warum-interne-links-wichtig-sind/">"#
        ),
        "{html}"
    );
}

#[test]
fn a_thin_tag_archive_is_noindex_but_followed() {
    let mut tag = SeoPage::new(
        PageKind::Tag,
        locale("en"),
        path("/tag/crawl-budget/"),
        "Crawl budget",
    );
    tag.item_count = Some(1);
    let tags = head_tags(&demo_site(), &tag).unwrap();
    insta::assert_snapshot!(tags.to_html());
}

#[test]
fn a_tag_archive_with_enough_posts_is_indexed() {
    let mut tag = SeoPage::new(PageKind::Tag, locale("en"), path("/tag/links/"), "Links");
    tag.item_count = Some(3);
    let html = head_tags(&demo_site(), &tag).unwrap().to_html();
    assert!(html.contains(r#"content="index, follow"#), "{html}");
}

#[test]
fn search_results_are_never_indexed() {
    let search = SeoPage::new(PageKind::Search, locale("en"), path("/search/"), "Search");
    let html = head_tags(&demo_site(), &search).unwrap().to_html();
    assert!(html.contains(r#"content="noindex, follow"#), "{html}");
}

#[test]
fn a_per_post_override_wins_over_the_template_and_the_policy() {
    let mut post = internal_links_post();
    post.seo_title = Some(MetaTemplate::parse("Internal links, explained {sep} {site}").unwrap());
    post.seo_description = Some(MetaTemplate::parse("Five minutes on internal links.").unwrap());
    post.canonical = Some(url("https://serpdad.com/blog/internal-links/"));
    let mut robots = RobotsDirectives::default();
    robots.indexing = Indexing::NoIndex;
    robots.following = Following::NoFollow;
    post.robots = Some(robots);
    let tags = head_tags(&demo_site(), &post).unwrap();
    insta::assert_snapshot!(tags.to_html());
}

#[test]
fn text_from_a_writer_is_escaped() {
    let mut post = internal_links_post();
    post.title = r#"Links & "anchors" <done right>"#.to_owned();
    let html = head_tags(&demo_site(), &post).unwrap().to_html();
    assert!(
        html.contains(
            "<title>Links &amp; &quot;anchors&quot; &lt;done right&gt; | Field Notes on SEO</title>"
        ),
        "{html}"
    );
}

#[test]
fn an_empty_title_leaves_no_dangling_separator() {
    let mut post = internal_links_post();
    post.title = String::new();
    let html = head_tags(&demo_site(), &post).unwrap().to_html();
    assert!(html.contains("<title>Field Notes on SEO</title>"), "{html}");
}
