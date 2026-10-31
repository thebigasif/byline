//! The fact registry, `facts.toml` (P7): `khalid lint`, `khalid build` and
//! the publish door hold every post to the facts the site keeps there.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// A post that passes every block and still gives the Pro plan's old price.
const OLD_PRICE: &str = "---
title: What the Pro plan costs
date: 2026-09-14
author: asif
description: What the Pro plan costs each month, and what every report in it includes.
---

The Pro plan costs 19 USD a month. See [how to pick link text](/blog/choosing-link-text/).
";

/// A post that gives the current price.
const NEW_PRICE: &str = "---
title: Why internal links matter
date: 2026-09-14
author: asif
description: Internal links help readers and search engines find the rest of your site.
---

The Pro plan costs 29 USD a month. See [what the plan costs](/blog/what-the-pro-plan-costs/).
";

/// The price, now 29 USD, checked long ago so every post that gives it is
/// due for a check.
const FACTS: &str = "[pro-plan-price]
value = \"29 USD a month\"
old = [\"19 USD a month\"]
checked = 2020-01-01
";

/// A site folder with the demo settings, `facts.toml` holding `facts`, and
/// the two price posts.
#[cfg(test)]
fn site(facts: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    fs::copy(settings, within(dir.path(), "site.toml")).unwrap();
    fs::write(within(dir.path(), "facts.toml"), facts).unwrap();
    let folder = within(dir.path(), "content/en/posts");
    fs::create_dir_all(&folder).unwrap();
    let posts = [
        ("2026-09-14-what-the-pro-plan-costs.md", OLD_PRICE),
        ("2026-09-14-why-internal-links-matter.md", NEW_PRICE),
    ];
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

/// The `(slug, subject)` of every finding of `rule` in a JSON report.
#[cfg(test)]
fn found(json: &Value, rule: &str) -> Vec<(String, String)> {
    let findings = json["findings"].as_array().unwrap();
    let of_rule = findings.iter().filter(|finding| finding["rule"] == rule);
    of_rule
        .map(|finding| {
            let slug = finding["slug"].as_str().unwrap_or_default().to_owned();
            let subject = finding["subject"].as_str().unwrap_or_default().to_owned();
            (slug, subject)
        })
        .collect()
}

#[test]
fn lint_warns_on_an_old_value_and_on_a_fact_due_for_a_check() {
    let site = site(FACTS);
    let output = khalid(site.path(), &["lint", "--json"]);
    let said = text(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{said}");
    let json: Value = serde_json::from_str(&said).unwrap();
    let old = found(&json, "fact-old-value");
    let old_price = (
        "what-the-pro-plan-costs".to_owned(),
        "19 USD a month".to_owned(),
    );
    assert_eq!(old, vec![old_price], "{json}");
    let due = found(&json, "fact-not-checked");
    let price_due = (
        "why-internal-links-matter".to_owned(),
        "pro-plan-price".to_owned(),
    );
    assert_eq!(due, vec![price_due], "{json}");
}

#[test]
fn lint_names_the_fix_in_sentences() {
    let site = site(FACTS);
    let output = khalid(site.path(), &["lint"]);
    let said = text(&output.stdout);
    assert!(said.contains("19 USD a month"), "{said}");
    assert!(said.contains("Replace the old value"), "{said}");
}

#[test]
fn a_broken_facts_file_stops_lint_and_build_with_a_plain_message() {
    let site = site("[pro-plan-price]\nvalue = \"29 USD a month\"\n");
    for args in [&["lint"][..], &["build"][..]] {
        let output = khalid(site.path(), args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        let said = text(&output.stderr);
        assert!(
            said.contains("facts.toml has a mistake on line 1"),
            "{said}"
        );
        assert!(said.contains("checked date"), "{said}");
    }
}

#[test]
fn build_leaves_out_an_old_value_when_the_site_makes_it_block() {
    let site = site(FACTS);
    let rules = "[rules]\nfact-old-value = \"block\"\n";
    fs::write(within(site.path(), "lint.toml"), rules).unwrap();
    let output = khalid(site.path(), &["build"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.contains("19 USD a month"), "{said}");
    assert!(said.contains("Left out 1 post"), "{said}");
}

#[test]
fn the_publish_door_refuses_a_draft_with_an_old_value_when_it_blocks() {
    let site = site(FACTS);
    let rules = "[rules]\nfact-old-value = \"block\"\n";
    fs::write(within(site.path(), "lint.toml"), rules).unwrap();
    let drafts = within(site.path(), "drafts/en");
    fs::create_dir_all(&drafts).unwrap();
    let draft = OLD_PRICE.replace("date: 2026-09-14\n", "slug: old-price\n");
    fs::write(within(&drafts, "old-price.md"), &draft).unwrap();
    let output = khalid(site.path(), &["publish", "old-price", "--json"]);
    let said = text(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "{said}");
    let answer: Value = serde_json::from_str(&said).unwrap();
    let old = found(&answer["lint"], "fact-old-value");
    assert_eq!(old.first().map(|(_, s)| s.as_str()), Some("19 USD a month"));
}

#[test]
fn a_site_without_facts_lints_as_before() {
    let site = site("");
    fs::remove_file(within(site.path(), "facts.toml")).unwrap();
    let output = khalid(site.path(), &["lint", "--json"]);
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    assert!(found(&json, "fact-old-value").is_empty(), "{json}");
    assert!(found(&json, "fact-not-checked").is_empty(), "{json}");
}
