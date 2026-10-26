//! Lists of posts: the front page and a collection, with pagination as plain
//! links that work with scripts off (BL-WEB-2).

use chrono::NaiveDate;
use khalid_render::{
    BodyHtml, Loading, PageBody, PageKind, Pagination, PostCard, RenderListing, Renderer,
};

use crate::common::{demo_site, meta};
use crate::html_check::assert_valid_page;
use crate::image::sitemap_image;
use crate::post::{link, person};

pub(crate) fn card(title: &str, slug: &str, day: u32, excerpt: &str) -> PostCard {
    PostCard {
        title: title.to_owned(),
        href: format!("/blog/{slug}/"),
        date: NaiveDate::from_ymd_opt(2026, 9, day).unwrap(),
        excerpt: excerpt.to_owned(),
        image: None,
        author: Some(person(
            "Lena Vogt",
            "lena-vogt",
            Some("Technical SEO editor"),
        )),
        category: Some(link("Technical SEO", "/blog/category/technical-seo/")),
    }
}

/// The first page of the demo blog, two posts per page.
pub(crate) fn first_page() -> RenderListing {
    let pages = vec!["/blog/".to_owned(), "/blog/page/2/".to_owned()];
    RenderListing {
        title: "Field notes on SEO".to_owned(),
        intro: Some(BodyHtml::from_sanitised(
            "<p>Short, practical posts for small teams who look after their own website.</p>"
                .to_owned(),
        )),
        posts: vec![
            card(
                "What a sitemap does, and what it does not",
                "what-a-sitemap-does",
                28,
                "A sitemap lists the pages you want search engines to know about.",
            ),
            card(
                "How to write a title tag",
                "how-to-write-a-title-tag",
                21,
                "The title tag is the blue link in search results.",
            ),
        ],
        pagination: Pagination::new(pages, 1),
    }
}

fn render(body: &PageBody) -> String {
    let meta = meta("Field notes on SEO", "/blog/", PageKind::Website);
    Renderer::new()
        .unwrap()
        .render(&demo_site("en"), &meta, body)
        .unwrap()
}

/// The inside of `main`, for snapshots that leave out the parts every page shares.
pub(crate) fn main_of(page: &str) -> &str {
    page.split("<main")
        .nth(1)
        .unwrap()
        .split("</main>")
        .next()
        .unwrap()
}

#[test]
fn home_page_in_english() {
    let page = render(&PageBody::Home(first_page()));
    assert_valid_page(&page);
    insta::assert_snapshot!(main_of(&page));
}

#[test]
fn collection_page_two_links_back_and_has_no_next() {
    let mut listing = first_page();
    listing.intro = None;
    listing.pagination = Pagination::new(vec!["/blog/".to_owned(), "/blog/page/2/".to_owned()], 2);
    let page = render(&PageBody::Collection(listing));
    assert_valid_page(&page);
    insta::assert_snapshot!(main_of(&page));
}

#[test]
fn pagination_needs_a_real_page_and_more_than_one() {
    let two = || vec!["/blog/".to_owned(), "/blog/page/2/".to_owned()];
    assert!(Pagination::new(two(), 0).is_none(), "pages count from 1");
    assert!(Pagination::new(two(), 3).is_none(), "there is no page 3");
    assert!(
        Pagination::new(vec!["/blog/".to_owned()], 1).is_none(),
        "one page needs no links"
    );
    assert!(Pagination::new(two(), 2).is_some());
}

#[test]
fn an_empty_list_says_so() {
    let mut listing = first_page();
    listing.posts.clear();
    listing.pagination = None;
    let page = render(&PageBody::Collection(listing));
    assert_valid_page(&page);
    assert!(page.contains("<p>There are no posts here yet.</p>"));
    assert!(!page.contains("pagination"));
}

#[test]
fn card_images_are_always_lazy() {
    let mut listing = first_page();
    for post in &mut listing.posts {
        post.image = Some(sitemap_image(Loading::Lazy));
    }
    let page = render(&PageBody::Home(listing));
    assert_valid_page(&page);
    assert_eq!(page.matches(r#"loading="lazy""#).count(), 2);
}
