//! `/archive/`: every published post across collections, newest first, with
//! pagination as plain links. It may carry `noindex` when it repeats the
//! front page exactly; the wiring decides that and the head prints it.

use khalid_render::{PageBody, PageKind, RenderListing, Renderer, Robots};

use crate::common::{demo_site, meta};
use crate::html_check::assert_valid_page;
use crate::listing::{first_page, main_of};

fn archive() -> RenderListing {
    let mut listing = first_page();
    listing.title = "Archive".to_owned();
    listing.intro = None;
    listing
}

fn render(robots: Robots) -> String {
    let mut meta = meta("Archive", "/blog/archive/", PageKind::Website);
    meta.robots = robots;
    Renderer::new()
        .unwrap()
        .render(&demo_site("en"), &meta, &PageBody::Archive(archive()))
        .unwrap()
}

#[test]
fn the_archive_lists_every_post_page_by_page() {
    let page = render(Robots::Index);
    assert_valid_page(&page);
    insta::assert_snapshot!(main_of(&page));
}

#[test]
fn an_archive_that_repeats_the_front_page_is_not_indexed() {
    let page = render(Robots::NoIndex);
    assert_valid_page(&page);
    assert!(page.contains(r#"<meta name="robots" content="noindex">"#));
    assert!(!render(Robots::Index).contains("noindex"));
}
