//! Related posts are chosen at build time from shared links, category and
//! tags, newest first on ties, and the same posts always give the same list.

use chrono::NaiveDate;
use khalid_render::{PostTopics, related};

#[cfg(test)]
fn post(slug: &str, day: u32, category: Option<&str>, tags: &[&str], links: &[&str]) -> PostTopics {
    PostTopics {
        slug: slug.to_owned(),
        date: NaiveDate::from_ymd_opt(2026, 9, day).unwrap(),
        category: category.map(str::to_owned),
        tags: tags.iter().map(|&tag| tag.to_owned()).collect(),
        links: links.iter().map(|&link| link.to_owned()).collect(),
    }
}

/// The demo blog: a sitemap post and the posts around it.
#[cfg(test)]
fn blog() -> Vec<PostTopics> {
    vec![
        post(
            "what-a-sitemap-does",
            28,
            Some("technical-seo"),
            &["sitemaps", "crawling"],
            &["robots-txt"],
        ),
        post("robots-txt", 10, Some("technical-seo"), &["crawling"], &[]),
        post(
            "crawl-budget",
            20,
            Some("technical-seo"),
            &["crawling"],
            &[],
        ),
        post(
            "internal-links",
            14,
            Some("on-page-seo"),
            &[],
            &["what-a-sitemap-does"],
        ),
        post("title-tags", 22, Some("on-page-seo"), &[], &[]),
        post("image-alt-text", 25, Some("on-page-seo"), &[], &[]),
        post("log-files", 5, Some("technical-seo"), &[], &[]),
        post("site-speed", 1, Some("technical-seo"), &[], &[]),
    ]
}

#[cfg(test)]
fn slugs(posts: &[PostTopics], chosen: &[usize]) -> Vec<String> {
    chosen
        .iter()
        .map(|&index| posts[index].slug.clone())
        .collect()
}

#[test]
fn the_posts_that_share_most_come_first_and_newest_wins_a_tie() {
    let posts = blog();
    let chosen = related(&posts[0], &posts, &[]);
    assert_eq!(
        slugs(&posts, &chosen),
        [
            // Linked, same category, a shared tag: 3 + 2 + 1.
            "robots-txt",
            // Same category and a shared tag: 2 + 1.
            "crawl-budget",
            // Links here: 3.
            "internal-links",
            // Same category only: 2, newest first.
            "log-files",
            "site-speed",
        ]
    );
}

#[test]
fn the_same_posts_in_another_order_give_the_same_list() {
    let posts = blog();
    let mut shuffled = blog();
    shuffled.reverse();
    let first = slugs(&posts, &related(&posts[0], &posts, &[]));
    let again = slugs(&shuffled, &related(&shuffled[7], &shuffled, &[]));
    assert_eq!(first, again);
}

#[test]
fn a_post_that_shares_little_is_filled_to_three_with_the_newest() {
    let posts = blog();
    let chosen = related(&posts[4], &posts, &[]);
    assert_eq!(
        slugs(&posts, &chosen),
        ["image-alt-text", "internal-links", "what-a-sitemap-does"]
    );
}

#[test]
fn a_pinned_list_wins_in_its_own_order_without_unknown_posts_or_itself() {
    let posts = blog();
    let pinned = [
        "title-tags".to_owned(),
        "no-such-post".to_owned(),
        "what-a-sitemap-does".to_owned(),
        "site-speed".to_owned(),
    ];
    let chosen = related(&posts[0], &posts, &pinned);
    assert_eq!(slugs(&posts, &chosen), ["title-tags", "site-speed"]);
}

#[test]
fn a_post_never_lists_itself_and_a_lone_post_lists_nothing() {
    let posts = blog();
    let chosen = related(&posts[0], &posts, &[]);
    assert!(!chosen.contains(&0));
    let alone = vec![post("only-post", 1, None, &[], &[])];
    assert_eq!(related(&alone[0], &alone, &[]), Vec::<usize>::new());
}
