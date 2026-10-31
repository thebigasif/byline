//! `khalid lint` reads each post as its page has it: its slug and routed
//! address, its wikilinked body, forms, focus keywords, collection and
//! topic cluster, and the home page's links, so the rules that need them
//! fire (audit 2, G5).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// A site folder with the demo settings, `routes` as `routes.toml` when
/// given, and `posts` as `(file under content/en, text)`.
#[cfg(test)]
fn site(routes: Option<&str>, posts: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    fs::copy(settings, within(dir.path(), "site.toml")).unwrap();
    if let Some(routes) = routes {
        fs::write(within(dir.path(), "routes.toml"), routes).unwrap();
    }
    let english = within(dir.path(), "content/en");
    for (name, text) in posts {
        let file = within(&english, name);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }
    dir
}

/// `name` inside `dir`, where `name` may hold `/`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.extend(name.split('/'));
    path
}

/// `khalid lint --json` on `site`, read.
#[cfg(test)]
fn lint(site: &Path) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .args(["lint", "--json"])
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&said).unwrap_or_else(|_| panic!("not JSON: {said}"))
}

/// The subjects of every finding of `rule` about the post `slug`.
#[cfg(test)]
fn found(report: &Value, rule: &str, slug: &str) -> Vec<String> {
    report["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("not a report: {report}"))
        .iter()
        .filter(|f| f["rule"] == rule && f["slug"] == slug)
        .map(|f| f["subject"].as_str().unwrap_or_default().to_owned())
        .collect()
}

/// A dated post with an author and a description, about `title`, whose
/// header ends with `extra` and whose body is `body`.
#[cfg(test)]
fn post(title: &str, extra: &str, body: &str) -> String {
    format!(
        "---\ntitle: {title}\ndate: 2026-09-14\nauthor: asif\n\
         description: A short guide for writers who want readers to find the rest of the site.\n\
         {extra}---\n\n{body}\n"
    )
}

#[test]
fn a_post_is_named_by_its_title_as_its_page_is_and_wikilinks_reach_it() {
    // The file name differs from the title: the page, and so the linter,
    // take the slug from the title.
    let site = site(
        None,
        &[
            (
                "posts/why-links.md",
                &post("Why links matter", "", "Read [[choosing-link-text]]."),
            ),
            (
                "posts/choosing-link-text.md",
                &post("Choosing link text", "", "Go back to [[why-links-matter]]."),
            ),
        ],
    );
    let report = lint(site.path());
    let none = Vec::<String>::new();
    assert_eq!(found(&report, "orphan-post", "choosing-link-text"), none);
    assert_eq!(found(&report, "orphan-post", "why-links-matter"), none);
    assert_eq!(found(&report, "no-internal-link", "why-links-matter"), none);
}

#[test]
fn a_form_whose_answers_go_nowhere_is_a_warning() {
    let body = "Write to us.\n\n:::form{id=\"contact\" button=\"Send\"}\n\
                - text name: Your name\n- email reply_to: Your email address\n:::";
    let site = site(
        None,
        &[("posts/contact-us.md", &post("Contact us", "", body))],
    );
    let report = lint(site.path());
    assert_eq!(
        found(&report, "form-without-destination", "contact-us"),
        ["contact"]
    );
}

#[test]
fn two_collections_at_the_same_addresses_are_a_warning() {
    let routes = "[collections.guides]\npermalink = \"/{slug}/\"\n";
    let site = site(
        Some(routes),
        &[
            (
                "posts/why-links-matter.md",
                &post("Why links matter", "", "A post."),
            ),
            ("guides/link-text.md", &post("Link text", "", "A guide.")),
        ],
    );
    let report = lint(site.path());
    assert_eq!(
        found(&report, "overlapping-collections", "link-text"),
        ["posts"]
    );
    assert_eq!(
        found(&report, "overlapping-collections", "why-links-matter"),
        ["guides"]
    );
}

#[test]
fn a_post_with_its_routed_address_is_found_by_links_to_that_address() {
    let routes = "[collections.posts]\npermalink = \"/{year}/{slug}/\"\n";
    let site = site(
        Some(routes),
        &[
            (
                "posts/why-links.md",
                &post(
                    "Why links matter",
                    "",
                    "Read [the guide](/blog/2026/link-text/).",
                ),
            ),
            (
                "posts/link-text.md",
                &post("Link text", "", "Read [why](/blog/2026/why-links-matter/)."),
            ),
        ],
    );
    let report = lint(site.path());
    assert_eq!(
        found(&report, "orphan-post", "link-text"),
        Vec::<String>::new()
    );
    assert_eq!(
        found(&report, "orphan-post", "why-links-matter"),
        Vec::<String>::new()
    );
}

#[test]
fn a_post_the_home_page_does_not_list_is_too_deep() {
    let undated = "---\ntitle: Notes for later\nauthor: asif\n---\n\nSee [[why-links-matter]].\n";
    let site = site(
        None,
        &[
            (
                "posts/why-links-matter.md",
                &post("Why links matter", "", "A post."),
            ),
            ("posts/notes-for-later.md", undated),
        ],
    );
    let report = lint(site.path());
    assert_eq!(
        found(&report, "too-deep", "why-links-matter"),
        Vec::<String>::new()
    );
    assert_eq!(found(&report, "too-deep", "notes-for-later"), [""]);
}

#[test]
fn a_focus_keyword_missing_from_each_place_is_a_warning_per_place() {
    let body = "Readers come back for clear writing.\n\n## How it starts\n\n\
                ![A tidy desk](/media/desk.jpg)";
    let extra = "focus_keyword: internal links\n";
    let site = site(
        None,
        &[("posts/writing-well.md", &post("Writing well", extra, body))],
    );
    let report = lint(site.path());
    for rule in [
        "focus-not-in-title",
        "focus-not-in-description",
        "focus-not-in-first-paragraph",
        "focus-not-in-subheading",
        "focus-not-in-address",
        "focus-not-in-image-alt",
    ] {
        assert_eq!(
            found(&report, rule, "writing-well"),
            ["internal links"],
            "{rule}"
        );
    }
}

/// `count` sentences of filler, ten words each, the first `uses` of them
/// holding "internal links".
#[cfg(test)]
fn filler(count: usize, uses: usize) -> String {
    (0..count)
        .map(|n| {
            if n < uses {
                "Good internal links help readers find the next useful page here."
            } else {
                "A clear sentence helps every reader keep going through the page."
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn a_focus_keyword_used_too_rarely_is_a_warning() {
    let extra = "focus_keyword: internal links\n";
    let body = filler(40, 0);
    let site = site(
        None,
        &[(
            "posts/internal-links.md",
            &post("Internal links", extra, &body),
        )],
    );
    let report = lint(site.path());
    assert_eq!(
        found(&report, "focus-too-rare", "internal-links"),
        ["internal links"]
    );
}

#[test]
fn a_focus_keyword_used_too_often_is_a_warning() {
    let extra = "focus_keyword: internal links\n";
    let body = filler(40, 40);
    let site = site(
        None,
        &[(
            "posts/internal-links.md",
            &post("Internal links", extra, &body),
        )],
    );
    let report = lint(site.path());
    assert_eq!(
        found(&report, "focus-too-often", "internal-links"),
        ["internal links"]
    );
}

#[test]
fn a_secondary_keyword_the_body_never_uses_is_a_warning() {
    let extra =
        "focus_keyword: internal links\nsecondary_keywords:\n  - link text\n  - site structure\n";
    let body = "Internal links need good link text.";
    let site = site(
        None,
        &[(
            "posts/internal-links.md",
            &post("Internal links", extra, body),
        )],
    );
    let report = lint(site.path());
    assert_eq!(
        found(&report, "secondary-keyword-unused", "internal-links"),
        ["site structure"]
    );
}

#[test]
fn a_cluster_member_that_never_links_to_its_pillar_is_a_warning() {
    let pillar = "cluster:\n  name: internal-links\n  role: pillar\n";
    let member = "cluster:\n  name: internal-links\n";
    let site = site(
        None,
        &[
            (
                "posts/internal-links.md",
                &post("Internal links", pillar, "See [[link-text]]."),
            ),
            (
                "posts/link-text.md",
                &post("Link text", member, "Start with [[internal-links]]."),
            ),
            (
                "posts/anchor-words.md",
                &post("Anchor words", member, "Read [[link-text]]."),
            ),
        ],
    );
    let report = lint(site.path());
    assert_eq!(
        found(&report, "cluster-without-pillar", "anchor-words"),
        ["internal-links"]
    );
    assert_eq!(
        found(&report, "cluster-without-pillar", "link-text"),
        Vec::<String>::new()
    );
}
