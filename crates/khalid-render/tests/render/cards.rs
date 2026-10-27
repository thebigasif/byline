//! Richer post cards: author name and photo, date, category, then title, on
//! every list. Switched off, a card shows its title, date and excerpt.

use khalid_render::{Feature, Features, PageBody, PageKind, Renderer};

use crate::common::{demo_site, meta};
use crate::html_check::assert_valid_page;
use crate::image::avatar_image;
use crate::listing::{first_page, home, main_of};
use crate::post::part;

fn render_home(features: Features) -> String {
    let mut site = demo_site("en");
    site.features = features;
    let mut listing = first_page();
    if let Some(author) = listing.posts[0].author.as_mut() {
        author.image = Some(avatar_image());
    }
    let meta = meta("Field Notes on SEO", "/blog/", PageKind::Website);
    Renderer::new()
        .unwrap()
        .render(&site, &meta, &PageBody::Home(home(listing)))
        .unwrap()
}

#[test]
fn a_card_shows_its_author_with_photo_date_and_category_before_its_title() {
    let page = render_home(Features::all());
    assert_valid_page(&page);
    insta::assert_snapshot!(part(
        main_of(&page),
        "<article class=\"card\">",
        "</article>"
    ));
}

#[test]
fn switched_off_a_card_is_title_date_and_excerpt() {
    let page = render_home(Features::all().without(Feature::RichCards));
    assert_valid_page(&page);
    assert!(!page.contains("card-meta"));
    assert!(!page.contains("<picture"));
    assert!(
        page.contains("</a></h2>\n<p><time datetime=\"2026-09-28\">28 September 2026</time></p>")
    );
}
