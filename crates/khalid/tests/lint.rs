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
fn lint_prints_each_post_below_100_with_its_score() {
    let site = site(&[
        ("2026-09-14-why-internal-links-matter.md", GOOD),
        ("choosing-link-text.md", UNDATED),
    ]);
    let output = khalid(site.path(), &["lint"]);
    let said = text(&output.stdout);
    assert!(said.contains("Scores out of 100:"), "{said}");
    assert!(said.contains("  en/choosing-link-text: 40\n"), "{said}");
    let one = "content/en/posts/choosing-link-text.md";
    let said = text(&khalid(site.path(), &["lint", one]).stdout);
    assert!(said.contains("Score: 40 out of 100.\n"), "{said}");
    assert!(!said.contains("Scores out of 100:"), "{said}");
}

#[test]
fn lint_json_scores_every_post_and_one_post_alone() {
    let site = site(&[
        ("2026-09-14-why-internal-links-matter.md", GOOD),
        ("choosing-link-text.md", UNDATED),
    ]);
    let output = khalid(site.path(), &["lint", "--json"]);
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    let scores = json["scores"].as_array().unwrap();
    assert_eq!(scores.len(), 2, "{json}");
    let undated = scores.iter().find(|s| s["slug"] == "choosing-link-text");
    let undated = undated.unwrap();
    assert_eq!(undated["locale"], "en");
    assert_eq!(undated["value"], 40);
    assert_eq!(undated["lost"][0]["rule"], "no-date");

    let one = "content/en/posts/choosing-link-text.md";
    let output = khalid(site.path(), &["lint", one, "--json"]);
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    assert_eq!(json["score"]["value"], 40, "{json}");
    assert_eq!(json.get("scores"), None);
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

/// A good post whose address has an accented letter.
const ACCENTED: &str = "---
title: Café menus that sell
slug: café-menus
date: 2026-09-14
author: asif
description: How a short menu on the café wall helps people choose and order faster.
---

Short menus work like [good link text](/blog/choosing-link-text/).
";

#[test]
fn lint_warns_about_an_accented_address_on_a_plain_address_site() {
    let site = site(&[("2026-09-14-cafe-menus.md", ACCENTED)]);
    let output = khalid(site.path(), &["lint", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let found = findings.iter().find(|f| f["rule"] == "unicode-slug");
    let found = found.unwrap_or_else(|| panic!("no unicode-slug in {json}"));
    assert_eq!(found["severity"], "warn");
    assert_eq!(found["subject"], "cafe-menus");
}

/// A good post that links to its own site over `http://`; the demo site's
/// address is `https://www.example.com`.
const INSECURE: &str = "---
title: Why internal links matter
date: 2026-09-14
author: asif
description: Internal links help readers and search engines find the rest of your site.
---

An internal link points to [how to pick link text](/blog/choosing-link-text/), and
an old one to [pruning posts](http://example.com/blog/pruning-old-posts/).
";

#[test]
fn lint_warns_about_a_link_to_the_own_site_over_http() {
    let site = site(&[("2026-09-14-why-internal-links-matter.md", INSECURE)]);
    let output = khalid(site.path(), &["lint", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let found = findings
        .iter()
        .find(|f| f["rule"] == "insecure-internal-link");
    let found = found.unwrap_or_else(|| panic!("no insecure-internal-link in {json}"));
    assert_eq!(found["severity"], "warn");
    assert_eq!(
        found["subject"],
        "http://example.com/blog/pruning-old-posts/"
    );
}

/// A good post written in three very long sentences of long words, under
/// a heading with no full stop.
const DENSE: &str = "---
title: Why internal links matter
date: 2026-09-14
author: asif
description: Internal links help readers and search engines find the rest of your site.
---

## Documentation at scale

Organizations implementing comprehensive internationalization strategies frequently underestimate the considerable administrative complexity introduced whenever multilingual documentation requires synchronized maintenance across numerous independently managed publication environments, particularly where contributors collaborate without [standardized terminology](/blog/choosing-link-text/).

Consequently, editorial departments responsible for authoritative reference material increasingly demand sophisticated verification procedures, systematic translation memories, comprehensive revision histories, and meticulously documented approval workflows before anything reaches publicly accessible destinations or downstream syndication partners.

Unfortunately, organizations attempting simultaneous modernization of infrastructure, governance, localization, accessibility, and performance commonly discover that interdependent requirements multiply exponentially, transforming seemingly straightforward improvements into lengthy, expensive, politically complicated undertakings that exhaust available engineering capacity and postpone measurable benefits indefinitely.
";

#[test]
fn lint_warns_about_a_post_that_is_hard_to_read() {
    let dense = site(&[("2026-09-14-why-internal-links-matter.md", DENSE)]);
    let output = khalid(dense.path(), &["lint", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let found = findings.iter().find(|f| f["rule"] == "hard-to-read");
    let found = found.unwrap_or_else(|| panic!("no hard-to-read in {json}"));
    assert_eq!(found["severity"], "warn");
    let easy = site(&[("2026-09-14-why-internal-links-matter.md", GOOD)]);
    let output = khalid(easy.path(), &["lint", "--json"]);
    assert!(!text(&output.stdout).contains("hard-to-read"));
}

/// A good post that moved: its old path redirects to it.
const MOVED: &str = "---
title: Choosing link text
date: 2026-09-10
author: asif
description: Good link text names the page it opens, so readers know where they go.
aliases:
  - /blog/link-text-tips/
---

Good link text names the page it opens, like [why links matter](/blog/why-internal-links-matter/).
";

/// A good post that still links to the moved post's old path.
const OLD_LINK: &str = "---
title: Why internal links matter
date: 2026-09-14
author: asif
description: Internal links help readers and search engines find the rest of your site.
---

An internal link points to [how to pick link text](/blog/link-text-tips/).
";

#[test]
fn lint_warns_about_a_link_to_an_old_address_of_a_moved_post() {
    let site = site(&[
        ("2026-09-10-choosing-link-text.md", MOVED),
        ("2026-09-14-why-internal-links-matter.md", OLD_LINK),
    ]);
    let output = khalid(site.path(), &["lint", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    let findings = json["findings"].as_array().unwrap();
    let found = findings.iter().find(|f| f["rule"] == "redirecting-link");
    let found = found.unwrap_or_else(|| panic!("no redirecting-link in {json}"));
    assert_eq!(found["slug"], "why-internal-links-matter");
    assert_eq!(found["subject"], "/blog/link-text-tips/");
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

/// A post with a figure that has a caption but no description.
const BARE_FIGURE: &str = "---
title: A door in Lisbon
date: 2026-09-14
author: asif
---

Read [why links matter](/blog/why-internal-links-matter/) first.

:::figure{src=\"/media/door.jpg\" alt=\"\"}
A *blue* door in Lisbon at noon.
:::
";

#[test]
fn lint_fix_describes_an_image_with_its_caption() {
    let site = site(&[("lisbon-door.md", BARE_FIGURE)]);
    let output = khalid(site.path(), &["lint", "--fix", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    let file = within(site.path(), "content/en/posts/lisbon-door.md");
    let described = BARE_FIGURE.replace("alt=\"\"}", "alt=\"A blue door in Lisbon at noon.\"}");
    assert_eq!(fs::read_to_string(file).unwrap(), described);
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    assert_eq!(json["fixed"][0]["rule"], "missing-alt");
    assert_eq!(json["fixed"][0]["subject"], "/media/door.jpg");
    let findings = json["findings"].as_array().unwrap();
    assert!(
        findings.iter().all(|f| f["rule"] != "missing-alt"),
        "{json}"
    );
}

/// A post with no `slug:`, so its address comes from a file name full of
/// filler words and a year.
const WORDY: &str = "---
title: Start a blog
date: 2026-09-14
author: asif
---

Read [why links matter](/blog/why-internal-links-matter/) first.
";

#[test]
fn lint_fix_cleans_the_address_and_keeps_the_old_one_working() {
    let site = site(&[("start-a-blog-in-2026.md", WORDY)]);
    let output = khalid(site.path(), &["lint", "--fix", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    let file = within(site.path(), "content/en/posts/start-a-blog-in-2026.md");
    let cleaned = WORDY.replace(
        "author: asif\n",
        "author: asif\nslug: start-blog\naliases:\n  - /blog/start-a-blog-in-2026/\n",
    );
    assert_eq!(fs::read_to_string(file).unwrap(), cleaned);
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    let rules: Vec<&Value> = json["fixed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| &f["rule"])
        .collect();
    assert_eq!(rules, ["slug-stopwords", "slug-date"], "{json}");
    let findings = json["findings"].as_array().unwrap();
    assert!(findings.iter().all(|f| f["slug"] == "start-blog"), "{json}");
}

#[test]
fn lint_fix_adds_to_aliases_and_replaces_the_slug_as_written() {
    let post = WORDY.replace(
        "author: asif\n",
        "author: asif\nslug: the-guide-2026\naliases:\n- /blog/old-guide/\n",
    );
    let site = site(&[("guide.md", &post)]);
    let output = khalid(site.path(), &["lint", "--fix"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let file = within(site.path(), "content/en/posts/guide.md");
    let cleaned = WORDY.replace(
        "author: asif\n",
        "author: asif\nslug: guide\naliases:\n- /blog/the-guide-2026/\n- /blog/old-guide/\n",
    );
    assert_eq!(fs::read_to_string(file).unwrap(), cleaned);
    let said = text(&output.stdout);
    assert!(
        said.contains("/blog/the-guide-2026/ now redirects"),
        "{said}"
    );
}

#[test]
fn lint_fix_leaves_an_address_another_post_already_has() {
    let site = site(&[("start-a-blog-in-2026.md", WORDY), ("start-blog.md", WORDY)]);
    let output = khalid(site.path(), &["lint", "--fix", "--json"]);
    let json: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
    assert_eq!(json["fixed"], Value::Array(Vec::new()), "{json}");
    let file = within(site.path(), "content/en/posts/start-a-blog-in-2026.md");
    assert_eq!(fs::read_to_string(file).unwrap(), WORDY);
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
