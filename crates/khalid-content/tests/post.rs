//! Reading a post's front matter. Title, date and author may be missing here,
//! because the linter blocks those with its own teaching message. Wrong types,
//! unknown fields and broken YAML are refused here, each with its own test and
//! the exact message.

use std::path::Path;

use khalid_content::parse_post;

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
        "posts/a.md, line 3, field \"colour\": unknown field `colour`, expected one of title, slug, date, author, category, description, translation"
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
