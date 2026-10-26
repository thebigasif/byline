//! Title and description templates per page type (task 1.13, KH-SEO-1).

use khalid_core::{AbsoluteUrl, Locale};
use khalid_seo::{MetaTemplate, PageKind, SeoPage, SeoSite, TemplatePair, UrlPath};

#[cfg(test)]
fn demo_site() -> SeoSite {
    let mut site = SeoSite::new(
        "Field Notes on SEO",
        AbsoluteUrl::parse("https://www.example.com").unwrap(),
        Locale::try_new("en").unwrap(),
    );
    site.tagline = String::from("Plain advice on search for small teams");
    site
}

#[cfg(test)]
fn page(kind: PageKind, title: &str) -> SeoPage {
    let mut page = SeoPage::new(
        kind,
        Locale::try_new("en").unwrap(),
        UrlPath::try_new("/why-internal-links-matter/").unwrap(),
        title,
    );
    page.excerpt = String::from("Internal links help readers find the rest of your site.");
    page.category = Some(String::from("On-page SEO"));
    page
}

#[test]
fn a_post_title_is_the_title_then_the_site_name() {
    let site = demo_site();
    let post = page(PageKind::Post, "Why internal links matter");
    let pair = site.templates.for_kind(PageKind::Post);
    assert_eq!(
        pair.title.render(&site, &post),
        "Why internal links matter | Field Notes on SEO"
    );
    assert_eq!(pair.description.render(&site, &post), post.excerpt);
}

#[test]
fn the_home_page_uses_the_site_name_and_tagline() {
    let site = demo_site();
    let home = page(PageKind::Home, "");
    let pair = site.templates.for_kind(PageKind::Home);
    assert_eq!(
        pair.title.render(&site, &home),
        "Field Notes on SEO | Plain advice on search for small teams"
    );
    assert_eq!(pair.description.render(&site, &home), site.tagline);
}

#[test]
fn a_site_template_for_one_page_type_leaves_the_others_alone() {
    let mut site = demo_site();
    site.templates.set(
        PageKind::Post,
        TemplatePair::new(
            MetaTemplate::parse("{title} {sep} {category} {sep} {site}").unwrap(),
            MetaTemplate::parse("{excerpt}").unwrap(),
        ),
    );
    let post = site.templates.for_kind(PageKind::Post);
    assert_eq!(
        post.title
            .render(&site, &page(PageKind::Post, "Anchor text")),
        "Anchor text | On-page SEO | Field Notes on SEO"
    );
    let about = site.templates.for_kind(PageKind::Page);
    assert_eq!(
        about.title.render(&site, &page(PageKind::Page, "About")),
        "About | Field Notes on SEO"
    );
}
