//! The input model: what a rule can read about a post and its site.

use khalid_core::{Locale, Slug};
use khalid_lint::{HeadingLevel, LintPost, LintSite, PageRef, SiteView};

#[test]
fn a_new_post_has_only_its_language_and_address() {
    let post = LintPost::new(
        Locale::try_new("de").unwrap(),
        Slug::try_new("warum-interne-links-zaehlen").unwrap(),
    );
    assert_eq!(post.locale.as_str(), "de");
    assert_eq!(post.title, None, "a missing title stays missing");
    assert_eq!(post.date, None);
    assert_eq!(post.author, None);
    assert_eq!(post.word_count, 0);
}

#[test]
fn a_site_lists_posts_and_pages_in_the_order_given() {
    let en = Locale::try_new("en").unwrap();
    let mut site = LintSite::default();
    for slug in ["why-links-matter", "anchor-text-basics"] {
        site.posts
            .push(LintPost::new(en.clone(), Slug::try_new(slug).unwrap()));
    }
    site.pages
        .push(PageRef::new(en, Slug::try_new("about").unwrap()));
    let view: &dyn SiteView = &site;
    let slugs: Vec<&str> = view.posts().iter().map(|post| post.slug.as_str()).collect();
    assert_eq!(slugs, ["why-links-matter", "anchor-text-basics"]);
    assert_eq!(view.pages().len(), 1);
    assert_eq!(view.glossary(), []);
}

#[test]
fn heading_levels_sort_from_the_main_headline_down() {
    assert!(HeadingLevel::H1 < HeadingLevel::H2);
    assert!(HeadingLevel::H5 < HeadingLevel::H6);
}
