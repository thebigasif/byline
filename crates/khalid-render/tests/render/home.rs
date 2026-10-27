//! The front page: sections from `site.toml` with "View all" links, or one
//! list of the newest posts, then a row of links to every topic.

use khalid_render::{
    Feature, HomeSectionPosts, Link, PageBody, PageKind, RenderHome, RenderSite, Renderer,
    TopicGroup,
};

use crate::common::{demo_site, meta};
use crate::html_check::assert_valid_page;
use crate::listing::{first_page, home, main_of};
use crate::post::site_without;

fn topic(name: &str, slug: &str) -> Link {
    Link {
        name: name.to_owned(),
        href: format!("/blog/category/{slug}/"),
    }
}

/// The demo front page with a Technical SEO section, an empty Guides
/// section, and the topics in two named groups.
fn sectioned() -> RenderHome {
    let listing = first_page();
    let mut front = home(listing.clone());
    front.sections = vec![
        HomeSectionPosts {
            title: "Technical SEO".to_owned(),
            href: "/blog/category/technical-seo/".to_owned(),
            posts: listing.posts,
        },
        HomeSectionPosts {
            title: "Guides".to_owned(),
            href: "/blog/guides/".to_owned(),
            posts: Vec::new(),
        },
    ];
    front.topics = vec![
        TopicGroup {
            name: Some("Search basics".to_owned()),
            links: vec![
                topic("Technical SEO", "technical-seo"),
                topic("On-page SEO", "on-page-seo"),
            ],
        },
        TopicGroup {
            name: Some("Growth".to_owned()),
            links: vec![topic("Link building", "link-building")],
        },
    ];
    front
}

fn render_on(site: &RenderSite, front: RenderHome) -> String {
    let meta = meta("Field Notes on SEO", "/blog/", PageKind::Website);
    Renderer::new()
        .unwrap()
        .render(site, &meta, &PageBody::Home(front))
        .unwrap()
}

#[test]
fn sections_with_view_all_links_then_topics_in_their_groups() {
    let page = render_on(&demo_site("en"), sectioned());
    assert_valid_page(&page);
    insta::assert_snapshot!(main_of(&page));
}

#[test]
fn without_sections_the_front_page_is_one_list_with_its_pages() {
    let mut front = sectioned();
    front.sections.clear();
    let page = render_on(&demo_site("en"), front);
    assert_valid_page(&page);
    assert!(!page.contains("home-section"));
    assert!(page.contains("<nav class=\"pagination\""));
    assert!(page.contains("<nav class=\"topics\" aria-label=\"Topics\">"));
}

#[test]
fn a_view_all_link_reads_in_the_page_language() {
    let page = render_on(&demo_site("de"), sectioned());
    assert!(page.contains(" Beiträge in Technical SEO</a>"));
}

#[test]
fn switched_off_there_are_no_topic_links() {
    let page = render_on(&site_without(Feature::TopicLinks), sectioned());
    assert_valid_page(&page);
    assert!(!page.contains("class=\"topics\""));
    assert!(page.contains("home-section"));
}
