//! What every page carries: language, direction, menus, the language switcher
//! and escaped values. The 404 page is the first page type.

use khalid_render::{PageBody, PageKind, PageMeta, RenderSite, Renderer, Robots};

use crate::common::{demo_site, locale, meta};

fn render(site: &RenderSite, meta: &PageMeta) -> String {
    Renderer::new()
        .unwrap()
        .render(site, meta, &PageBody::NotFound)
        .unwrap()
}

pub(crate) fn render_not_found(tag: &str) -> String {
    let mut meta = meta("Page not found", "/blog/missing/", PageKind::Website);
    meta.robots = Robots::NoIndex;
    meta.alternates.clear();
    meta.x_default = None;
    render(&demo_site(tag), &meta)
}

#[test]
fn not_found_page_in_german() {
    let page = render_not_found("de");
    assert!(page.contains(r#"<html lang="de" dir="ltr">"#));
    assert!(page.contains("<h1>Diese Seite gibt es nicht</h1>"));
    assert!(page.contains(r##"<a class="skip-link" href="#main">Zum Inhalt springen</a>"##));
    assert!(page.contains(r#"<meta name="robots" content="noindex">"#));
}

#[test]
fn arabic_pages_run_right_to_left() {
    let mut site = demo_site("en");
    site.locale = locale("ar");
    let page = render(
        &site,
        &meta("Page not found", "/blog/missing/", PageKind::Website),
    );
    assert!(page.contains(r#"<html lang="ar" dir="rtl">"#));
}

#[test]
fn the_current_menu_link_is_marked() {
    let page = render(
        &demo_site("en"),
        &meta("Glossary", "/blog/glossary/", PageKind::Website),
    );
    assert!(page.contains(r#"<a href="/blog/glossary/" aria-current="page">Glossary</a>"#));
    assert!(page.contains(r#"<a href="/blog/">Posts</a>"#));
}

#[test]
fn the_language_switcher_links_every_version() {
    let page = render(
        &demo_site("en"),
        &meta("Glossary", "/blog/glossary/", PageKind::Website),
    );
    assert!(page.contains(
        r#"<a href="https://www.example.com/blog/de/glossary/" hreflang="de" lang="de">Deutsch</a>"#
    ));
    assert!(page.contains(r#"hreflang="x-default" href="https://www.example.com/blog/glossary/""#));
}

#[test]
fn values_are_escaped_for_html() {
    let mut site = demo_site("en");
    site.name = r#"<script>alert("hi")</script>"#.to_owned();
    let page = render(
        &site,
        &meta(
            "Tom & Jerry \"quoted\"",
            "/blog/missing/",
            PageKind::Website,
        ),
    );
    assert!(!page.contains("<script>"), "the site name must be escaped");
    assert!(page.contains("&lt;script&gt;"));
    assert!(page.contains(r#"content="Tom &amp; Jerry &quot;quoted&quot;""#));
}
