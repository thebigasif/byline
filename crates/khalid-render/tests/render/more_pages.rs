//! The author page, a plain page, and the tag and category lists.

use chrono::NaiveDate;
use khalid_render::{
    BodyHtml, Link, Loading, PageBody, PageKind, RenderAuthor, RenderPage, Renderer,
};

use crate::common::{demo_site, meta};
use crate::html_check::assert_valid_page;
use crate::image::sitemap_image;
use crate::listing::{first_page, main_of};
use crate::post::person;

fn render(body: &PageBody, tag: &str) -> String {
    let meta = meta("A page", "/blog/a-page/", PageKind::Website);
    Renderer::new()
        .unwrap()
        .render(&demo_site(tag), &meta, body)
        .unwrap()
}

/// Lena Vogt from the demo site, with one of her posts.
pub(crate) fn lena() -> RenderAuthor {
    RenderAuthor {
        name: "Lena Vogt".to_owned(),
        role: Some("Technical SEO editor".to_owned()),
        image: None,
        bio: BodyHtml::from_sanitised(
            "<p>Lena looks after the technical side: sitemaps, page speed and everything a crawler sees.</p>"
                .to_owned(),
        ),
        profiles: vec![Link {
            name: "Mastodon".to_owned(),
            href: "https://mastodon.example/@lena".to_owned(),
        }],
        posts: first_page().posts,
        pagination: None,
        others: vec![person("Asif", "asif", Some("Founder, SerpDad"))],
    }
}

#[test]
fn author_page_in_english() {
    let page = render(&PageBody::Author(Box::new(lena())), "en");
    assert_valid_page(&page);
    insta::assert_snapshot!(main_of(&page));
}

#[test]
fn author_page_in_german_names_the_posts_the_german_way() {
    let page = render(&PageBody::Author(Box::new(lena())), "de");
    assert_valid_page(&page);
    assert!(page.contains(r#"<h2 id="author-posts">Beiträge von Lena Vogt</h2>"#));
}

#[test]
fn an_author_photo_is_lazy_and_reserves_its_space() {
    let mut author = lena();
    author.image = Some(sitemap_image(Loading::Lazy));
    author.posts.clear();
    let page = render(&PageBody::Author(Box::new(author)), "en");
    assert_valid_page(&page);
    assert!(page.contains(r#"loading="lazy""#));
    assert!(page.contains("<p>There are no posts here yet.</p>"));
}

#[test]
fn plain_page_in_english() {
    let about = RenderPage {
        title: "About this blog".to_owned(),
        updated: NaiveDate::from_ymd_opt(2026, 10, 1),
        body: BodyHtml::from_sanitised(
            "<p>Field Notes on SEO is written by the team at SerpDad.</p>".to_owned(),
        ),
    };
    let page = render(&PageBody::Page(about), "en");
    assert_valid_page(&page);
    insta::assert_snapshot!(main_of(&page));
}

#[test]
fn tag_and_category_pages_are_lists() {
    for body in [
        PageBody::Tag(first_page()),
        PageBody::Category(first_page()),
    ] {
        let page = render(&body, "en");
        assert_valid_page(&page);
        assert!(page.contains("<h1>Field notes on SEO</h1>"));
        assert!(page.contains(r#"rel="next""#));
    }
}
