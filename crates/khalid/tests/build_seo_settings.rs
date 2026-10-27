//! `khalid build` applies the site-wide SEO tables of `site.toml` to every
//! page (audit G11): title templates, the share image, the Twitter account
//! and verification codes (KH-SEO-1, KH-SEO-2, KH-SEO-7).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

#[cfg(test)]
fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = at(into, &entry.file_name().to_string_lossy());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Titles, social and verification, as a site owner writes them.
const HEAD_TABLES: &str = r#"
[seo.titles]
separator = "·"
tagline = "Plain notes on search"

[seo.titles.post]
title = "{title} {sep} {site} {sep} {author}"

[seo.social]
twitter = "serpdad"

[seo.social.image]
url = "https://www.example.com/media/share.png"
alt = "The Field Notes logo on a white card"

[seo.verification]
google = "aBc123-_"
bing = "F00D42"
"#;

/// The demo site with `settings` added to the end of its `site.toml`,
/// built into a new folder. Returns the site and the output folder.
#[cfg(test)]
fn built_with(settings: &str) -> (TempDir, TempDir) {
    let site = tempfile::tempdir().unwrap();
    let demo = at(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        "../../fixtures/demo-site",
    );
    copy_dir(&demo, site.path());
    let toml = at(site.path(), "site.toml");
    let text = fs::read_to_string(&toml).unwrap();
    fs::write(&toml, format!("{text}{settings}")).unwrap();
    let out = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.path().to_str().unwrap(), "build", "--out"])
        .arg(out.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    (site, out)
}

/// The tags of the page at `rel` that the head settings change: the title,
/// the description, verification, Open Graph and Twitter.
#[cfg(test)]
fn head_lines(out: &TempDir, rel: &str) -> String {
    let page = fs::read_to_string(at(out.path(), rel)).unwrap();
    page.lines()
        .filter(|line| {
            line.starts_with("<title>")
                || line.starts_with("<meta name=\"description\"")
                || line.starts_with("<meta name=\"google")
                || line.starts_with("<meta name=\"msvalidate")
                || line.starts_with("<meta property=\"og:")
                || line.starts_with("<meta name=\"twitter:")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_head_tables_reach_the_home_page() {
    let (_site, out) = built_with(HEAD_TABLES);
    insta::assert_snapshot!(head_lines(&out, "index.html"));
}

#[test]
fn a_post_title_follows_its_template() {
    let (_site, out) = built_with(HEAD_TABLES);
    let tags = head_lines(&out, "why-internal-links-matter/index.html");
    let title = "<title>Why internal links matter · Field Notes on SEO · Asif</title>";
    assert!(tags.contains(title), "{tags}");
    assert!(
        tags.contains(r#"<meta name="twitter:site" content="@serpdad">"#),
        "{tags}"
    );
    assert!(!tags.contains("google-site-verification"), "{tags}");
}
