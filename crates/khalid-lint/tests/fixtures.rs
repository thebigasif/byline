//! One fixture per message (BL-TST-4).
//!
//! Each folder under `fixtures/` holds `post.toml`, the post under test, and
//! `expected.txt`, its findings one per line as `<severity> <rule>`, followed
//! by `: <subject>` when the finding names a link, image or phrase, and by
//! `(suggest: a, b)` when it suggests posts to link to. An
//! optional `site.toml` holds the rest of the site. Every rule in the
//! catalogue must have at least one fixture.
//!
//! Site rules have theirs under `fixtures/site/<rule>/`: a `site.toml` and an
//! `expected.txt` with every site finding, each line starting with the
//! post's `locale/slug: `.
//!
//! A `post.toml` with `extends = "clean"` starts from `fixtures/clean/` and
//! writes only what differs: its keys replace the clean post's, and the keys
//! listed in `unset` are removed.
//!
//! Any fixture may hold a `lint.toml`, for example to switch off rules it is
//! not about. Today is 2026-10-26 for every fixture.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use chrono::{TimeZone, Utc};
use khalid_core::FixedClock;
use khalid_lint::{Catalogue, Finding, LintConfig, LintPost, LintSite, Linter, Severity};
use toml::Table;

/// The linter for `folder`, with its `lint.toml` if it has one.
#[cfg(test)]
fn linter_for(folder: &Path) -> Linter {
    let text = fixture_file(folder, "lint.toml").unwrap_or_default();
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 26, 9, 0, 0).unwrap());
    Linter::new(&LintConfig::parse(&text).unwrap(), &clock).unwrap()
}

#[cfg(test)]
fn fixture_file(folder: &Path, name: &str) -> std::io::Result<String> {
    let mut path = folder.to_path_buf();
    path.push(name);
    fs::read_to_string(path)
}

/// The post in `folder`, with `extends` and `unset` applied.
#[cfg(test)]
fn post_table(folder: &Path) -> Table {
    let mut own: Table = toml::from_str(&fixture_file(folder, "post.toml").unwrap()).unwrap();
    let Some(base) = own.remove("extends") else {
        return own;
    };
    let mut base_folder = folder.to_path_buf();
    base_folder.pop();
    base_folder.push(base.as_str().unwrap());
    let mut post = post_table(&base_folder);
    let unset = own
        .remove("unset")
        .map(|keys| keys.try_into::<Vec<String>>().unwrap());
    for key in unset.unwrap_or_default() {
        post.remove(&key);
    }
    post.extend(own);
    post
}

#[cfg(test)]
fn line(finding: &Finding) -> String {
    let severity = match finding.severity() {
        Severity::Block => "block",
        Severity::Warn => "warn",
        Severity::Off => "off",
    };
    let mut line = match &finding.location().subject {
        Some(subject) => format!("{severity} {}: {subject}", finding.rule()),
        None => format!("{severity} {}", finding.rule()),
    };
    if !finding.suggestions().is_empty() {
        line = format!("{line} (suggest: {})", finding.suggestions().join(", "));
    }
    line
}

#[cfg(test)]
fn run(folder: &Path) -> (Vec<String>, Vec<String>) {
    let linter = linter_for(folder);
    let read = |name: &str| fixture_file(folder, name);
    let post: LintPost = post_table(folder)
        .try_into()
        .unwrap_or_else(|error| panic!("{}: {error}", folder.display()));
    let mut site: LintSite = read("site.toml")
        .map(|text| toml::from_str(&text).unwrap())
        .unwrap_or_default();
    site.posts.push(post.clone());
    let got = linter
        .lint_post(&post, &site)
        .findings()
        .iter()
        .map(line)
        .collect();
    (got, expected(folder))
}

/// A site fixture: the site rules over `site.toml`, each line starting with
/// the post's `locale/slug`.
#[cfg(test)]
fn run_site(folder: &Path) -> (Vec<String>, Vec<String>) {
    let linter = linter_for(folder);
    let site: LintSite = toml::from_str(&fixture_file(folder, "site.toml").unwrap())
        .unwrap_or_else(|error| panic!("{}: {error}", folder.display()));
    let report = linter.site_health(&site);
    let got = report.findings().iter().map(|finding| {
        let at = finding.location();
        format!("{}/{}: {}", at.locale, at.slug, line(finding))
    });
    (got.collect(), expected(folder))
}

#[cfg(test)]
fn expected(folder: &Path) -> Vec<String> {
    let text = fixture_file(folder, "expected.txt").unwrap();
    text.lines()
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The fixture folders directly under `root`, sorted.
#[cfg(test)]
fn folders(root: &str) -> Vec<std::path::PathBuf> {
    let entries = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path());
    let mut folders: Vec<_> = entries.filter(|path| !path.ends_with("site")).collect();
    folders.sort();
    folders
}

#[test]
fn every_fixture_produces_exactly_its_expected_findings() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");
    let site_root = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/site");
    let posts = folders(root).into_iter().map(|f| (run(&f), f));
    let sites = folders(site_root).into_iter().map(|f| (run_site(&f), f));
    let mut covered = BTreeSet::new();
    for ((got, want), folder) in posts.chain(sites) {
        assert_eq!(got, want, "fixture {}", folder.display());
        let name = folder.file_name().unwrap().to_string_lossy().into_owned();
        covered.insert(name);
    }
    let catalogue = Catalogue::builtin().unwrap();
    for (rule, _) in &catalogue {
        assert!(covered.contains(rule.as_str()), "{rule} has no fixture");
    }
}
