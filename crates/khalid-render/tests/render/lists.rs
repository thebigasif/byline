//! The list pages `routes.toml` adds: a channel, a date archive, and the
//! HTML sitemap that links every listed page by section (G12).

use khalid_render::{
    Link, PageBody, PageKind, RenderListing, RenderSitemap, Renderer, SitemapGroup, SitemapSection,
};

use crate::common::{demo_site, meta};
use crate::html_check::assert_valid_page;
use crate::listing::{first_page, main_of};

fn render(title: &str, path: &str, body: &PageBody) -> String {
    let meta = meta(title, path, PageKind::Website);
    Renderer::new()
        .unwrap()
        .render(&demo_site("en"), &meta, body)
        .unwrap()
}

fn titled(title: &str) -> RenderListing {
    let mut listing = first_page();
    title.clone_into(&mut listing.title);
    listing.intro = None;
    listing
}

fn link(name: &str, href: &str) -> Link {
    Link {
        name: name.to_owned(),
        href: href.to_owned(),
    }
}

#[test]
fn a_channel_lists_its_posts_like_any_list() {
    let body = PageBody::Channel(titled("Featured"));
    let page = render("Featured", "/blog/featured/", &body);
    assert_valid_page(&page);
    insta::assert_snapshot!(main_of(&page));
}

#[test]
fn a_date_archive_lists_the_posts_of_its_month() {
    let body = PageBody::Date(titled("September 2026"));
    let page = render("September 2026", "/blog/2026/09/", &body);
    assert_valid_page(&page);
    assert!(page.contains("<h1>September 2026</h1>"), "{page}");
    assert!(page.contains(r#"href="/blog/page/2/""#), "{page}");
}

#[test]
fn the_sitemap_links_each_section_under_its_heading() {
    let sitemap = RenderSitemap {
        title: "Sitemap".to_owned(),
        groups: vec![
            SitemapGroup {
                section: SitemapSection::Posts,
                links: vec![link("How to write a title tag", "/blog/title-tags/")],
            },
            SitemapGroup {
                section: SitemapSection::Pages,
                links: vec![link("About", "/blog/about/")],
            },
            SitemapGroup {
                section: SitemapSection::Archives,
                links: vec![link("Technical SEO", "/blog/category/technical-seo/")],
            },
        ],
    };
    let page = render("Sitemap", "/blog/sitemap/", &PageBody::Sitemap(sitemap));
    assert_valid_page(&page);
    insta::assert_snapshot!(main_of(&page));
}

#[test]
fn an_empty_sitemap_says_so() {
    let sitemap = RenderSitemap {
        title: "Sitemap".to_owned(),
        groups: Vec::new(),
    };
    let page = render("Sitemap", "/blog/sitemap/", &PageBody::Sitemap(sitemap));
    assert_valid_page(&page);
    assert!(
        page.contains("<p>This blog has no pages to list yet.</p>"),
        "{page}"
    );
}
