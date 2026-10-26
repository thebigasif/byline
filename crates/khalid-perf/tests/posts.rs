//! Every generated post reads like a real one and is the same everywhere.

use std::collections::BTreeSet;

use khalid_content::{Post, Rendered, parse_post, render};
use khalid_core::{Locale, SlugMode};
use khalid_perf::Fixture;

#[cfg(test)]
fn fixture(posts: usize) -> Fixture {
    Fixture::new(posts).unwrap()
}

#[cfg(test)]
fn read(fixture: Fixture, index: usize) -> (Post, Rendered) {
    let path = fixture.post_path(index);
    let post = parse_post(&path, &fixture.post_text(index)).unwrap();
    let locale = Locale::try_new("en").unwrap();
    let rendered = render(&post.body, &locale, SlugMode::Ascii).unwrap();
    (post, rendered)
}

#[test]
fn every_post_is_long_enough_and_links_only_to_posts_that_exist() {
    let site = fixture(40);
    let slugs: BTreeSet<String> = (0..40)
        .map(|index| read(site, index).0.front.slug.unwrap())
        .map(|slug| format!("/blog/{slug}/"))
        .collect();
    assert_eq!(slugs.len(), 40, "every slug is different");
    for index in 0..40 {
        let (post, rendered) = read(site, index);
        assert!(post.front.title.is_some() && post.front.description.is_some());
        assert!(post.front.author.is_some() && post.front.date.is_some());
        assert!(rendered.words >= 300, "post {index}");
        assert_eq!(rendered.headings.len(), 4, "post {index}");
        let (internal, outside): (Vec<_>, Vec<_>) =
            rendered.links.iter().partition(|link| link.internal);
        assert_eq!((internal.len(), outside.len()), (3, 1), "post {index}");
        for link in internal {
            assert!(slugs.contains(&link.href), "{}", link.href);
        }
    }
}

#[test]
fn ten_thousand_posts_have_ten_thousand_files() {
    let site = Fixture::TEN_THOUSAND;
    let paths: BTreeSet<_> = (0..site.posts())
        .map(|index| site.post_path(index))
        .collect();
    assert_eq!(paths.len(), 10_000);
}

#[test]
fn the_same_post_is_the_same_text_and_another_post_is_not() {
    let site = Fixture::THOUSAND;
    assert_eq!(site.post_text(512), fixture(1_000).post_text(512));
    assert_ne!(site.post_text(512), site.post_text(513));
}

#[test]
fn a_lone_post_links_nowhere_inside_the_site() {
    let (_, rendered) = read(fixture(1), 0);
    assert!(rendered.links.iter().all(|link| !link.internal));
}

#[test]
fn no_posts_is_not_a_fixture() {
    assert_eq!(Fixture::new(0), None);
    assert_eq!(Fixture::THOUSAND.posts(), 1_000);
    assert_eq!(Fixture::TEN_THOUSAND.posts(), 10_000);
}
