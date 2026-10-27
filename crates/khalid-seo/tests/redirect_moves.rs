//! The automatic 301 when a page's address changes, and the nearest page
//! for an address that answered 404 (KH-SEO-8).

#![cfg(test)]

use std::collections::BTreeMap;

use khalid_seo::{
    Destination, Move, Redirect, RedirectRule, UrlPath, moved_addresses, nearest_page,
};

fn path(text: &str) -> UrlPath {
    UrlPath::try_new(text).unwrap()
}

fn addresses(pairs: &[(&'static str, &str)]) -> BTreeMap<&'static str, UrlPath> {
    pairs.iter().map(|(file, at)| (*file, path(at))).collect()
}

fn moved(from: &str, to: &str) -> RedirectRule {
    let to = Destination::Path(path(to));
    RedirectRule::new(
        path(from),
        Redirect::Move {
            kind: Move::Permanent,
            to,
        },
    )
}

#[test]
fn a_page_whose_address_changed_gets_a_permanent_move() {
    let before = addresses(&[
        ("posts/links.md", "/blog/internal-links/"),
        ("posts/sitemaps.md", "/blog/what-a-sitemap-does/"),
        ("posts/old.md", "/blog/retired-tool/"),
    ]);
    let after = addresses(&[
        ("posts/links.md", "/blog/why-internal-links-matter/"),
        ("posts/sitemaps.md", "/blog/what-a-sitemap-does/"),
        ("posts/new.md", "/blog/robots-txt/"),
    ]);
    assert_eq!(
        moved_addresses(&before, &after),
        [moved(
            "/blog/internal-links/",
            "/blog/why-internal-links-matter/"
        )]
    );
}

#[test]
fn an_old_address_another_page_now_holds_gets_no_move() {
    let before = addresses(&[
        ("posts/a.md", "/blog/link-building/"),
        ("posts/b.md", "/blog/link-building-2025/"),
    ]);
    let after = addresses(&[
        ("posts/a.md", "/blog/link-building-2024/"),
        ("posts/b.md", "/blog/link-building/"),
    ]);
    assert_eq!(
        moved_addresses(&before, &after),
        [moved("/blog/link-building-2025/", "/blog/link-building/")]
    );
}

fn pages() -> Vec<UrlPath> {
    [
        "/blog/",
        "/blog/why-internal-links-matter/",
        "/blog/what-a-sitemap-does/",
        "/blog/robots-txt/",
        "/blog/de/warum-interne-links-wichtig-sind/",
    ]
    .into_iter()
    .map(path)
    .collect()
}

fn nearest(missing: &str) -> Option<String> {
    let pages = pages();
    nearest_page(&path(missing), &pages).map(ToString::to_string)
}

#[test]
fn the_page_sharing_the_most_words_is_nearest() {
    assert_eq!(
        nearest("/blog/internal-links/"),
        Some("/blog/why-internal-links-matter/".to_owned())
    );
    assert_eq!(
        nearest("/blog/2019/05/what-sitemaps-do/"),
        Some("/blog/what-a-sitemap-does/".to_owned())
    );
}

#[test]
fn a_misspelt_address_finds_its_page() {
    assert_eq!(
        nearest("/blog/robot-txt/"),
        Some("/blog/robots-txt/".to_owned())
    );
    assert_eq!(
        nearest("/blog/why-internl-links-mater/"),
        Some("/blog/why-internal-links-matter/".to_owned())
    );
}

#[test]
fn an_address_like_no_page_has_no_nearest() {
    assert_eq!(nearest("/blog/wp-login.php"), None);
    assert_eq!(nearest("/blog/xyz/"), None);
    assert_eq!(nearest_page(&path("/blog/robots-txt/"), &[]), None);
}
