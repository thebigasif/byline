//! `/authors/`: every author with photo, role and post count. Each author
//! page ends with the other authors.

use khalid_render::{
    AuthorEntry, Feature, PageBody, PageKind, RenderAuthors, RenderSite, Renderer,
};

use crate::common::{demo_site, meta};
use crate::html_check::assert_valid_page;
use crate::image::avatar_image;
use crate::listing::main_of;
use crate::post::{person, site_without};

fn authors() -> RenderAuthors {
    let mut lena = person("Lena Vogt", "lena-vogt", Some("Technical SEO editor"));
    lena.image = Some(avatar_image());
    RenderAuthors {
        title: "Authors".to_owned(),
        authors: vec![
            AuthorEntry {
                person: lena,
                posts: 12,
            },
            AuthorEntry {
                person: person("Asif", "asif", Some("Founder, SerpDad")),
                posts: 1,
            },
        ],
    }
}

fn render(site: &RenderSite, body: &PageBody) -> String {
    let meta = meta("Authors", "/blog/authors/", PageKind::Website);
    Renderer::new().unwrap().render(site, &meta, body).unwrap()
}

#[test]
fn the_authors_page_lists_each_author_with_role_and_post_count() {
    let page = render(&demo_site("en"), &PageBody::Authors(authors()));
    assert_valid_page(&page);
    insta::assert_snapshot!(main_of(&page));
}

#[test]
fn post_counts_read_in_the_page_language() {
    let page = render(&demo_site("de"), &PageBody::Authors(authors()));
    assert!(page.contains("<p>12 Beiträge</p>"));
    assert!(page.contains("<p>1 Beitrag</p>"));
}

#[test]
fn a_blog_without_authors_says_so() {
    let mut empty = authors();
    empty.authors.clear();
    let page = render(&demo_site("en"), &PageBody::Authors(empty));
    assert_valid_page(&page);
    assert!(page.contains("<p>This blog has no authors yet.</p>"));
}

#[test]
fn switched_off_an_author_page_does_not_list_the_others() {
    let lena = crate::more_pages::lena();
    let shown = render(&demo_site("en"), &PageBody::Author(Box::new(lena.clone())));
    assert!(shown.contains("<h2 id=\"other-authors\">Other authors</h2>"));
    let off = render(
        &site_without(Feature::AuthorsPage),
        &PageBody::Author(Box::new(lena)),
    );
    assert_valid_page(&off);
    assert!(!off.contains("other-authors"));
}
