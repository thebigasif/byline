//! A new site's first post, as a new user writes it: the linter's messages
//! can be followed on a site with one post (KH-LNT-1, KH-LNT-2).

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
