//! Reading a post's front matter. Title, date and author may be missing here,
//! because the linter blocks those with its own teaching message. Wrong types,
//! unknown fields and broken YAML are refused here, each with its own test and
//! the exact message.

use std::path::Path;

use khalid_content::parse_post;
use khalid_core::{AbsoluteUrl, Slug};

const GOOD: &str = "---
title: Why internal links matter
slug: why-internal-links-matter
date: 2026-09-14
author: asif
category: on-page-seo
description: Internal links help readers and search engines.
translation: internal-links
---

An internal link is a link from one page to another.
";

#[cfg(test)]
fn message(text: &str) -> String {
    parse_post(Path::new("posts/a.md"), text)
        .unwrap_err()
        .to_string()
}

#[test]
fn reads_every_field_and_keeps_the_body_exactly() {
    let post = parse_post(Path::new("posts/a.md"), GOOD).unwrap();
    let front = &post.front;
    assert_eq!(front.title.as_deref(), Some("Why internal links matter"));
    assert_eq!(
        front.slug.as_ref().map(khalid_core::Slug::as_str),
        Some("why-internal-links-matter")
    );
    assert_eq!(
        front.date.map(|d| d.to_string()).as_deref(),
        Some("2026-09-14")
    );
    assert_eq!(
        front.author.as_ref().map(khalid_core::Slug::as_str),
        Some("asif")
    );
    assert_eq!(
        front.category.as_ref().map(khalid_core::Slug::as_str),
        Some("on-page-seo")
    );
    assert_eq!(front.translation.as_deref(), Some("internal-links"));
    assert_eq!(
        post.body,
        "\nAn internal link is a link from one page to another.\n"
    );
}

#[test]
fn allows_the_fields_the_linter_checks_to_be_missing() {
    let post = parse_post(
        Path::new("posts/a.md"),
        "---\ncategory: on-page-seo\n---\nText.\n",
    )
    .unwrap();
    assert!(post.front.title.is_none() && post.front.date.is_none() && post.front.author.is_none());
}

#[test]
fn refuses_an_unknown_field() {
    assert_eq!(
        message("---\ntitle: A\ncolour: blue\n---\n"),
        "posts/a.md, line 3, field \"colour\": unknown field `colour`, expected one of title, slug, date, author, category, description, translation, aliases, featured, robots, canonical, reviewed_by, related, focus_keyword, secondary_keywords, access, template, password_hash, head_code"
    );
}

#[test]
fn reads_the_focus_keyword_and_secondary_keywords_as_one_or_a_list() {
    let text = "---\nfocus_keyword: internal links\nsecondary_keywords:\n  - link text\n  - orphan posts\n---\n";
    let post = parse_post(Path::new("posts/a.md"), text).unwrap();
    assert_eq!(post.front.focus_keyword.as_deref(), Some("internal links"));
    assert_eq!(post.front.secondary_keywords, ["link text", "orphan posts"]);
    let one = parse_post(
        Path::new("posts/a.md"),
        "---\nsecondary_keywords: link text\n---\n",
    )
    .unwrap();
    assert_eq!(one.front.secondary_keywords, ["link text"]);
    let plain = parse_post(Path::new("posts/a.md"), GOOD).unwrap();
    assert!(plain.front.focus_keyword.is_none() && plain.front.secondary_keywords.is_empty());
}

#[test]
fn refuses_a_blank_focus_keyword() {
    assert_eq!(
        message("---\nfocus_keyword: \"  \"\n---\n"),
        "posts/a.md, line 2, field \"focus_keyword\": a keyword needs at least one word, such as internal links; remove the field if the post has none"
    );
}

#[test]
fn refuses_a_blank_secondary_keyword() {
    assert_eq!(
        message("---\nsecondary_keywords: [link text, \"\"]\n---\n"),
        "posts/a.md, line 2, field \"secondary_keywords\": a keyword needs at least one word, such as internal links; remove the field if the post has none"
    );
}

#[test]
fn refuses_more_than_four_secondary_keywords() {
    assert_eq!(
        message("---\nsecondary_keywords: [a, b, c, d, e]\n---\n"),
        "posts/a.md, line 2, field \"secondary_keywords\": a post has at most 4 secondary keywords, and this lists 5; keep the 4 closest to the focus keyword"
    );
}

#[test]
fn refuses_a_date_that_is_not_a_real_day() {
    assert_eq!(
        message("---\ndate: 2026-13-40\n---\n"),
        "posts/a.md, line 2, field \"date\": the date must be a real day written as YYYY-MM-DD, such as 2026-09-14"
    );
}

#[test]
fn refuses_a_slug_that_breaks_the_rules() {
    assert_eq!(
        message("---\nslug: Why Links\n---\n"),
        "posts/a.md, line 2, field \"slug\": the slug has 'W' at byte 0, but only a to z, 0 to 9 and hyphens are allowed"
    );
}

#[test]
fn refuses_a_field_with_the_wrong_kind_of_value() {
    assert_eq!(
        message("---\ntitle: [one, two]\n---\n"),
        "posts/a.md, line 2, field \"title\": this field must be plain text, not a list or a group"
    );
}

#[test]
fn refuses_front_matter_that_is_not_yaml() {
    assert_eq!(
        message("---\ntitle: A\n  oops: indented\n---\n"),
        "posts/a.md, line 3, field \"oops\": the line has a second colon; put the value in quotes if it needs one"
    );
}

#[test]
fn refuses_a_file_without_front_matter() {
    assert_eq!(
        message("# Just a heading\n"),
        "posts/a.md does not start with front matter between two --- lines"
    );
}

#[test]
fn refuses_front_matter_that_is_never_closed() {
    assert_eq!(
        message("---\ntitle: A\n\nText.\n"),
        "posts/a.md opens its front matter with --- but never closes it"
    );
}

#[test]
fn reads_aliases_and_featured() {
    let text =
        "---\ntitle: A\naliases:\n  - /2019/05/old-title/\n  - /blog/?p=12\nfeatured: true\n---\n";
    let post = parse_post(Path::new("posts/a.md"), text).unwrap();
    assert_eq!(post.front.aliases, ["/2019/05/old-title/", "/blog/?p=12"]);
    assert!(post.front.featured);
    let plain = parse_post(Path::new("posts/a.md"), "---\ntitle: A\n---\n").unwrap();
    assert!(plain.front.aliases.is_empty() && !plain.front.featured);
}

#[test]
fn reads_robots_and_canonical() {
    let text = "---\ntitle: A\nrobots: noindex, nofollow\ncanonical: https://moz.com/blog/internal-links\n---\n";
    let post = parse_post(Path::new("posts/a.md"), text).unwrap();
    assert_eq!(
        post.front
            .robots
            .map(|robots| robots.to_string())
            .as_deref(),
        Some("noindex, nofollow")
    );
    assert_eq!(
        post.front.canonical.as_ref().map(AbsoluteUrl::as_str),
        Some("https://moz.com/blog/internal-links")
    );
    let plain = parse_post(Path::new("posts/a.md"), "---\ntitle: A\n---\n").unwrap();
    assert!(plain.front.robots.is_none() && plain.front.canonical.is_none());
}

#[test]
fn refuses_a_robots_word_it_does_not_know() {
    assert_eq!(
        message("---\ntitle: A\nrobots: noindex, nofolow\n---\n"),
        "posts/a.md, line 3, field \"robots\": \"nofolow\" is not a robots directive; use index, noindex, follow, nofollow, noarchive, nosnippet, noimageindex, notranslate, max-snippet, max-image-preview or max-video-preview"
    );
}

#[test]
fn refuses_a_canonical_that_is_not_a_full_address() {
    assert_eq!(
        message("---\ntitle: A\ncanonical: /blog/internal-links/\n---\n"),
        "posts/a.md, line 3, field \"canonical\": the address is not a full URL starting with https://"
    );
}

#[test]
fn refuses_an_alias_that_is_not_a_path() {
    assert_eq!(
        message("---\naliases:\n  - old-title\n---\n"),
        "posts/a.md, line 3, field \"aliases\": \"old-title\" is not a path; an alias is an old address on this site starting with /, such as /2019/05/old-title/"
    );
}

#[test]
fn reads_reviewers_and_related_posts_as_one_slug_or_a_list() {
    let text = "---\ntitle: A\nreviewed_by: lena-vogt\nrelated:\n  - what-a-sitemap-does\n  - fixing-slow-pages\n---\n";
    let post = parse_post(Path::new("posts/a.md"), text).unwrap();
    assert_eq!(
        post.front.reviewed_by,
        [Slug::try_new("lena-vogt").unwrap()]
    );
    assert_eq!(
        post.front.related,
        [
            Slug::try_new("what-a-sitemap-does").unwrap(),
            Slug::try_new("fixing-slow-pages").unwrap()
        ]
    );
    let plain = parse_post(Path::new("posts/a.md"), "---\ntitle: A\n---\n").unwrap();
    assert!(plain.front.reviewed_by.is_empty() && plain.front.related.is_empty());
}

#[test]
fn refuses_a_reviewer_that_is_not_a_slug() {
    assert_eq!(
        message("---\nreviewed_by:\n  - Lena Vogt\n---\n"),
        "posts/a.md, line 3, field \"reviewed_by\": \"Lena Vogt\" is not a slug; name the file in authors/ or posts/ without .md, such as lena-vogt"
    );
}

#[test]
fn refuses_more_than_five_related_posts() {
    assert_eq!(
        message("---\nrelated: [a, b, c, d, e, f]\n---\n"),
        "posts/a.md, line 2, field \"related\": a post shows at most 5 related posts, and this lists 6; keep the 5 that matter most"
    );
}

#[test]
fn refuses_a_broken_block_with_the_line_in_the_file() {
    assert_eq!(
        message(
            "---\ntitle: A\n---\n\nIntro.\n\n:::figur{src=\"a.jpg\" alt=\"A\"}\nCaption.\n:::\n"
        ),
        "posts/a.md, line 7: there is no block called figur; the closest name is figure"
    );
}
