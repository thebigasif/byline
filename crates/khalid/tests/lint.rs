//! `khalid lint`: every rule over a site folder, in sentences or JSON, with
//! exit code 1 on a block (KH-LNT-3, RULES.md BL-CLI-1 to BL-CLI-3).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// A post that passes every block: title, date, author and an internal link.
const GOOD: &str = "---
title: Why internal links matter
date: 2026-09-14
author: asif
description: Internal links help readers and search engines find the rest of your site.
---

An internal link points to [how to pick link text](/blog/choosing-link-text/).
";

/// The same post with no date, which blocks publishing.
const UNDATED: &str = "---
title: Choosing link text
author: asif
---

Good link text names the page it opens, like [why links matter](/blog/why-internal-links-matter/).
";

/// A site folder with the demo settings and `posts` as `(file name, text)`.
#[cfg(test)]
fn site(posts: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    fs::copy(settings, within(dir.path(), "site.toml")).unwrap();
    let folder = within(dir.path(), "content/en/posts");
    fs::create_dir_all(&folder).unwrap();
    for (name, text) in posts {
        fs::write(within(&folder, name), text).unwrap();
    }
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

#[test]
fn lint_passes_a_site_with_only_warnings() {
    let site = site(&[("2026-09-14-why-internal-links-matter.md", GOOD)]);
    let output = khalid(site.path(), &["lint"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.contains("0 blocks"), "{said}");
}

#[test]
fn lint_blocks_a_post_with_no_date_and_says_why() {
    let site = site(&[
        ("2026-09-14-why-internal-links-matter.md", GOOD),
        ("choosing-link-text.md", UNDATED),
    ]);
    let output = khalid(site.path(), &["lint"]);
    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stdout);
    assert!(
        said.contains("en/choosing-link-text: blocks publishing"),
        "{said}"
    );
    assert!(said.contains("This post has no date."), "{said}");
    assert!(said.contains("Add `date:` to the header."), "{said}");
}

#[test]
fn lint_json_prints_the_report_and_still_exits_1() {
    let site = site(&[("choosing-link-text.md", UNDATED)]);
    let output = khalid(site.path(), &["lint", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    assert_eq!(json["blocks"], true);
    let first = &json["findings"][0];
    assert_eq!(first["rule"], "no-date");
    assert_eq!(first["slug"], "choosing-link-text");
    assert_eq!(first["guide"], "post-dates");
}

#[test]
fn lint_checks_only_the_post_it_is_given() {
    let site = site(&[
        ("2026-09-14-why-internal-links-matter.md", GOOD),
        ("choosing-link-text.md", UNDATED),
    ]);
    let good = "content/en/posts/2026-09-14-why-internal-links-matter.md";
    let output = khalid(site.path(), &["lint", good, "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .all(|f| f["slug"] == "why-internal-links-matter")
    );
}

#[test]
fn lint_names_a_post_it_cannot_find() {
    let site = site(&[("choosing-link-text.md", UNDATED)]);
    let output = khalid(site.path(), &["lint", "content/en/posts/missing.md"]);
    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(said.contains("content/en/posts/missing.md"), "{said}");
}

#[test]
fn a_mistake_in_lint_toml_names_its_line() {
    let site = site(&[("2026-09-14-why-internal-links-matter.md", GOOD)]);
    fs::write(
        within(site.path(), "lint.toml"),
        "[rules]\nno-dates = \"off\"\n",
    )
    .unwrap();
    let output = khalid(site.path(), &["lint"]);
    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(said.contains("line 2"), "{said}");
    assert!(said.contains("no-dates"), "{said}");
}

#[test]
fn lint_toml_turns_a_block_into_a_warning() {
    let site = site(&[("choosing-link-text.md", UNDATED)]);
    fs::write(
        within(site.path(), "lint.toml"),
        "[rules]\nno-date = \"warn\"\n",
    )
    .unwrap();
    let output = khalid(site.path(), &["lint"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
}

/// A post whose `####` heading skips a level, with the two blank lines
/// before `## Setup` that a repair must leave as they are.
const JUMPY: &str = "---
title: Setting up a blog
date: 2026-09-14
author: asif
---

Read [why links matter](/blog/why-internal-links-matter/) first.


## Setup

#### Pick a folder

Any empty folder works.

## Writing

###### Last words
";

#[test]
fn lint_fix_puts_headings_in_order_and_leaves_the_rest() {
    let site = site(&[("setting-up.md", JUMPY)]);
    let file = within(site.path(), "content/en/posts/setting-up.md");
    let before = text(&khalid(site.path(), &["lint"]).stdout);
    assert!(before.contains("Rule heading-order"), "{before}");
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        JUMPY,
        "lint alone wrote"
    );

    let output = khalid(site.path(), &["lint", "--fix"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let in_order = JUMPY
        .replace("#### Pick", "### Pick")
        .replace("###### Last", "### Last");
    assert_eq!(fs::read_to_string(&file).unwrap(), in_order);
    let said = text(&output.stdout);
    assert!(
        said.contains("Fixed content/en/posts/setting-up.md"),
        "{said}"
    );
    assert!(
        said.contains("\"Pick a folder\" is now a level 3"),
        "{said}"
    );
    assert!(said.contains("\"Last words\" is now a level 3"), "{said}");
    assert!(!said.contains("Rule heading-order"), "{said}");
    assert!(said.contains("Fixed 2 problems."), "{said}");
}

#[test]
fn lint_fix_json_lists_repairs_and_still_exits_1_on_a_block() {
    let undated_jumpy = JUMPY.replace("date: 2026-09-14\n", "");
    let site = site(&[("setting-up.md", &undated_jumpy)]);
    let output = khalid(site.path(), &["lint", "--fix", "--json"]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stdout));
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    assert_eq!(json["blocks"], true);
    let fixed = json["fixed"].as_array().unwrap();
    assert_eq!(fixed.len(), 2, "{json}");
    assert_eq!(fixed[0]["path"], "content/en/posts/setting-up.md");
    assert_eq!(fixed[0]["rule"], "heading-order");
    assert_eq!(fixed[0]["subject"], "Pick a folder");
    let findings = json["findings"].as_array().unwrap();
    assert!(findings.iter().any(|f| f["rule"] == "no-date"), "{json}");
    assert!(
        findings.iter().all(|f| f["rule"] != "heading-order"),
        "{json}"
    );
}

#[test]
fn lint_fix_leaves_a_rule_that_is_off() {
    let site = site(&[("setting-up.md", JUMPY)]);
    fs::write(
        within(site.path(), "lint.toml"),
        "[rules]\nheading-order = \"off\"\n",
    )
    .unwrap();
    let output = khalid(site.path(), &["lint", "--fix", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    assert_eq!(json["fixed"], Value::Array(Vec::new()));
    let file = within(site.path(), "content/en/posts/setting-up.md");
    assert_eq!(fs::read_to_string(file).unwrap(), JUMPY);
}

#[test]
fn lint_fix_says_when_there_is_nothing_to_fix() {
    let site = site(&[("2026-09-14-why-internal-links-matter.md", GOOD)]);
    let output = khalid(site.path(), &["lint", "--fix"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.contains("Nothing needed a safe fix."), "{said}");
}

#[test]
fn lint_needs_a_site_folder() {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("lint")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("needs a site folder"));
}
