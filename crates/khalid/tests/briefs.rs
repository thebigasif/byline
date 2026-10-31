//! A brief per post, `briefs/<language>/<slug>.toml` (P6): `khalid lint`,
//! `khalid build` and the publish door hold each post to its brief.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// A post that passes every block, asks one of its brief's two questions,
/// never says "anchor text" and links to one of its brief's two pages.
const POST: &str = "---
title: Why internal links matter
date: 2026-09-14
author: asif
description: Internal links help readers and search engines find the rest of your site.
---

Internal links help readers find the next useful post on your site.

## What internal links are

A link between two pages of one site. See [how to pick link text](/blog/choosing-link-text/).
";

/// The brief for [`POST`], with a note the linter never reads.
const BRIEF: &str =
    "questions = [\"What are internal links?\", \"How many internal links should a post have?\"]
terms = [\"next useful post\", \"anchor text\"]
links = [\"/blog/choosing-link-text/\", \"/blog/orphan-pages/\"]
notes = \"For readers new to SEO.\"
";

/// A site folder with the demo settings, [`POST`] in English, and `brief`
/// as its brief.
#[cfg(test)]
fn site(brief: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    fs::copy(settings, within(dir.path(), "site.toml")).unwrap();
    let posts = within(dir.path(), "content/en/posts");
    fs::create_dir_all(&posts).unwrap();
    fs::write(
        within(&posts, "2026-09-14-why-internal-links-matter.md"),
        POST,
    )
    .unwrap();
    let briefs = within(dir.path(), "briefs/en");
    fs::create_dir_all(&briefs).unwrap();
    fs::write(within(&briefs, "why-internal-links-matter.toml"), brief).unwrap();
    fs::write(
        within(&briefs, "README.md"),
        "Briefs for the English posts.\n",
    )
    .unwrap();
    dir
}

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

#[cfg(test)]
fn khalid(site: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .args(args)
        .output()
        .unwrap()
}

#[cfg(test)]
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The subject of every finding of `rule` in a JSON report, in order.
#[cfg(test)]
fn found(json: &Value, rule: &str) -> Vec<String> {
    let findings = json["findings"].as_array().unwrap();
    let of_rule = findings.iter().filter(|finding| finding["rule"] == rule);
    of_rule
        .map(|finding| finding["subject"].as_str().unwrap_or_default().to_owned())
        .collect()
}

#[test]
fn lint_warns_about_each_part_of_the_brief_the_post_misses() {
    let site = site(BRIEF);
    let output = khalid(site.path(), &["lint", "--json"]);
    let said = text(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{said}");
    let json: Value = serde_json::from_str(&said).unwrap();
    assert_eq!(
        found(&json, "brief-question-unanswered"),
        ["How many internal links should a post have?"],
        "{json}"
    );
    assert_eq!(
        found(&json, "brief-term-missing"),
        ["anchor text"],
        "{json}"
    );
    assert_eq!(
        found(&json, "brief-link-missing"),
        ["/blog/orphan-pages/"],
        "{json}"
    );
}

#[test]
fn lint_names_the_fix_in_sentences() {
    let site = site(BRIEF);
    let output = khalid(site.path(), &["lint"]);
    let said = text(&output.stdout);
    assert!(said.contains("anchor text"), "{said}");
    assert!(said.contains("remove it from the brief"), "{said}");
}

#[test]
fn a_broken_brief_stops_lint_and_build_with_a_plain_message() {
    let site = site("terms = [\"anchor text\"]\nquestion = [\"Why?\"]\n");
    for args in [&["lint"][..], &["build"][..]] {
        let output = khalid(site.path(), args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        let said = text(&output.stderr);
        assert!(
            said.contains("briefs/en/why-internal-links-matter.toml has a mistake on line 2"),
            "{said}"
        );
    }
}

#[test]
fn a_brief_outside_a_language_folder_is_refused() {
    let site = site(BRIEF);
    fs::write(within(site.path(), "briefs/stray.toml"), BRIEF).unwrap();
    let output = khalid(site.path(), &["lint"]);
    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(
        said.contains("briefs/stray.toml is not in a language folder"),
        "{said}"
    );
}

#[test]
fn build_leaves_out_an_unanswered_question_when_the_site_makes_it_block() {
    let site = site(BRIEF);
    let rules = "[rules]\nbrief-question-unanswered = \"block\"\n";
    fs::write(within(site.path(), "lint.toml"), rules).unwrap();
    let output = khalid(site.path(), &["build"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.contains("How many internal links"), "{said}");
    assert!(said.contains("Left out 1 post"), "{said}");
}

#[test]
fn the_publish_door_refuses_a_draft_that_misses_its_brief_when_it_blocks() {
    let site = site(BRIEF);
    let rules = "[rules]\nbrief-link-missing = \"block\"\n";
    fs::write(within(site.path(), "lint.toml"), rules).unwrap();
    let drafts = within(site.path(), "drafts/en");
    fs::create_dir_all(&drafts).unwrap();
    let draft = POST.replace("date: 2026-09-14\n", "slug: links-draft\n");
    fs::write(within(&drafts, "links-draft.md"), &draft).unwrap();
    let briefs = within(site.path(), "briefs/en");
    fs::write(within(&briefs, "links-draft.toml"), BRIEF).unwrap();
    let output = khalid(site.path(), &["publish", "links-draft", "--json"]);
    let said = text(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{said}");
    let answer: Value = serde_json::from_str(&said).unwrap();
    assert_eq!(
        found(&answer["lint"], "brief-link-missing"),
        ["/blog/orphan-pages/"]
    );
}

#[cfg(unix)]
#[test]
fn a_brief_linked_from_outside_the_site_folder_is_refused() {
    let site = site(BRIEF);
    let outside = tempfile::tempdir().unwrap();
    let secret = within(outside.path(), "elsewhere.toml");
    fs::write(&secret, BRIEF).unwrap();
    let link = within(site.path(), "briefs/en/linked.toml");
    std::os::unix::fs::symlink(&secret, link).unwrap();
    let output = khalid(site.path(), &["lint"]);
    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(
        said.contains("could not read briefs/en/linked.toml"),
        "{said}"
    );
}

#[test]
fn a_post_without_a_brief_lints_as_before() {
    let site = site(BRIEF);
    fs::remove_dir_all(within(site.path(), "briefs")).unwrap();
    let output = khalid(site.path(), &["lint", "--json"]);
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    for rule in [
        "brief-question-unanswered",
        "brief-term-missing",
        "brief-link-missing",
    ] {
        assert!(found(&json, rule).is_empty(), "{rule}: {json}");
    }
}
