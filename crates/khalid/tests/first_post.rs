//! A new site's first post, as a new user writes it: the linter's messages
//! can be followed on a site with one post, and once they are, the site
//! passes lint and audit with no warning (KH-LNT-1, KH-LNT-2, KH-LNT-5).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// A first post that names `sam` as its author and links to the home page,
/// the only other page a first post can link to.
const FIRST_POST: &str = "---
title: Why internal links matter
date: 2026-10-31
author: sam
---

Start with [the home page of this blog](/).
";

/// The header fields and body a writer adds to the post `khalid new`
/// wrote, after reading each lint message once.
const FOLLOWED: &str = "description: Internal links help readers and search engines find the rest of your site. Here is how to add them without overthinking it.
focus_keyword: internal links
---

Internal links are links from one page on your site to another page on the same site. They sound too simple to matter. They matter a lot, and this post explains why and how to add them.

## Readers use internal links to keep going

Someone who finishes a post is interested. If the post ends without pointing anywhere, they leave. One link to a related post gives them a next step, and gives you a second page view. Over a year, those second page views add up to a real share of your traffic, and they cost you nothing but a sentence.

A good link sits where the reader needs it. If a paragraph mentions a term you explain elsewhere, link the term there. If a post ends with a question, link the post that answers it.

## Search engines use internal links to find pages

A crawler finds new pages by following links. A post that nothing links to is hard to find and looks unimportant. Google says as much in its [guide to links](https://developers.google.com/search/docs/crawling-indexing/links-crawlable): it finds most pages through links from pages it already knows.

The words you link with matter too. They tell the search engine what the page on the other end is about, in the same way they tell the reader.

## How to add internal links well

- Link from new posts to older ones, and go back to add links from older posts to new ones.
- Use link words that say what the reader gets, never \"click here\".
- Use the same link words for the same page every time, so one phrase means one page.

Start with one link per post. While this blog is new, every post can link back to [the home page of this blog](/), where readers find everything else.

## What to do next

Pick your three oldest posts. Add one link from each to a newer post on a related topic. Then do the same in the other direction. It takes ten minutes, and it is the cheapest thing you can do for a small site this week.
";

/// The About page the `no-about-page` warning asks for.
const ABOUT: &str = "---
title: About this blog
slug: about
description: Who writes this blog about search, and how to reach them.
---

Sam Lee writes this blog about search for people who run their own website. Write to sam@example.com.
";

#[cfg(test)]
fn khalid(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .output()
        .unwrap()
}

/// `rest` inside `dir`, its folders made.
#[cfg(test)]
fn file_in(dir: &Path, rest: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(rest);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    path
}

/// A site made by `khalid init` in a temporary folder.
#[cfg(test)]
fn new_site() -> (tempfile::TempDir, String) {
    let temp = tempfile::tempdir().unwrap();
    let folder = temp.path().to_str().unwrap().to_owned();
    let made = khalid(&[
        "init",
        &folder,
        "--address",
        "https://blog.example.com",
        "--yes",
    ]);
    assert_eq!(made.status.code(), Some(0), "{made:?}");
    (temp, folder)
}

/// The rules `khalid lint --json` reports, each as `rule` or `rule: subject`.
#[cfg(test)]
fn lint_rules(folder: &str) -> Vec<String> {
    let output = khalid(&["--site", folder, "lint", "--json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let body: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("{stdout}"));
    let findings = body["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("{stdout}"));
    findings
        .iter()
        .map(|finding| {
            let rule = finding["rule"].as_str().unwrap();
            match finding["subject"].as_str() {
                Some(subject) => format!("{rule}: {subject}"),
                None => rule.to_owned(),
            }
        })
        .collect()
}

#[test]
fn a_first_post_is_never_blocked_or_orphaned_for_having_no_other_post() {
    let (temp, folder) = new_site();
    let post = file_in(
        temp.path(),
        "content/en/posts/2026-10-31-why-internal-links-matter.md",
    );
    fs::write(post, FIRST_POST).unwrap();
    let rules = lint_rules(&folder);
    assert!(
        !rules.iter().any(|rule| rule == "no-internal-link"),
        "{rules:?}"
    );
    assert!(!rules.iter().any(|rule| rule == "orphan-post"), "{rules:?}");
}

#[test]
fn an_author_with_no_profile_is_reported_until_the_profile_is_written() {
    let (temp, folder) = new_site();
    let post = file_in(
        temp.path(),
        "content/en/posts/2026-10-31-why-internal-links-matter.md",
    );
    fs::write(post, FIRST_POST).unwrap();
    let rules = lint_rules(&folder);
    assert!(
        rules.contains(&"author-without-profile: sam".to_owned()),
        "{rules:?}"
    );

    let profile = file_in(temp.path(), "content/en/authors/sam.md");
    fs::write(profile, "---\nname: Sam Lee\nslug: sam\n---\n").unwrap();
    let rules = lint_rules(&folder);
    assert!(
        !rules
            .iter()
            .any(|rule| rule.starts_with("author-without-profile")),
        "{rules:?}"
    );
}

/// The issues `khalid audit --offline --json` finds at `severity`, each as
/// `check url`.
#[cfg(test)]
fn audit_issues(folder: &str, severity: &str) -> Vec<String> {
    let output = khalid(&["--site", folder, "audit", "--offline", "--json"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let body: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("{stdout}"));
    let issues = body["issues"]
        .as_array()
        .unwrap_or_else(|| panic!("{stdout}"));
    issues
        .iter()
        .filter(|issue| issue["severity"] == severity)
        .map(|issue| {
            let check = issue["check"].as_str().unwrap();
            format!("{check} {}", issue["url"].as_str().unwrap())
        })
        .collect()
}

#[test]
fn a_new_site_whose_first_post_follows_every_message_passes_lint_and_audit() {
    let (temp, folder) = new_site();
    let args = ["--site", &folder, "new", "Why internal links matter"];
    let made = khalid(&[&args[..], &["--author", "Sam Lee"]].concat());
    assert_eq!(made.status.code(), Some(0), "{made:?}");
    let posts = file_in(temp.path(), "content/en/posts/.");
    let post = fs::read_dir(&posts)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let written = fs::read_to_string(&post).unwrap();
    let header = written.strip_suffix("---\n\n").unwrap();
    fs::write(&post, format!("{header}{FOLLOWED}")).unwrap();
    fs::write(file_in(temp.path(), "content/en/pages/about.md"), ABOUT).unwrap();

    assert_eq!(lint_rules(&folder), Vec::<String>::new());
    assert_eq!(audit_issues(&folder, "error"), Vec::<String>::new());
    assert_eq!(audit_issues(&folder, "warning"), Vec::<String>::new());
}
