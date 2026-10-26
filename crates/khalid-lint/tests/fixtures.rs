//! One fixture per message (BL-TST-4).
//!
//! Each folder under `fixtures/` holds `post.toml`, the post under test, and
//! `expected.txt`, its findings one per line as `<severity> <rule>`, followed
//! by `: <subject>` when the finding names a link, image or phrase. An
//! optional `site.toml` holds the rest of the site. Every rule in the
//! catalogue must have at least one fixture.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use khalid_lint::{Catalogue, Finding, LintConfig, LintPost, LintSite, Linter, Severity};

#[cfg(test)]
fn line(finding: &Finding) -> String {
    let severity = match finding.severity() {
        Severity::Block => "block",
        Severity::Warn => "warn",
        Severity::Off => "off",
    };
    match &finding.location().subject {
        Some(subject) => format!("{severity} {}: {subject}", finding.rule()),
        None => format!("{severity} {}", finding.rule()),
    }
}

#[cfg(test)]
fn run(folder: &Path, linter: &Linter) -> (Vec<String>, Vec<String>) {
    let read = |name: &str| {
        let mut path = folder.to_path_buf();
        path.push(name);
        fs::read_to_string(path)
    };
    let post: LintPost = toml::from_str(&read("post.toml").unwrap())
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
    let want = read("expected.txt")
        .unwrap()
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    (got, want)
}

#[test]
fn every_fixture_produces_exactly_its_expected_findings() {
    let linter = Linter::new(&LintConfig::default()).unwrap();
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures");
    let mut covered = BTreeSet::new();
    let mut folders: Vec<_> = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    folders.sort();
    for folder in &folders {
        let (got, want) = run(folder, &linter);
        assert_eq!(got, want, "fixture {}", folder.display());
        let name = folder.file_name().unwrap().to_string_lossy().into_owned();
        covered.insert(name.split('.').next().unwrap_or_default().to_owned());
    }
    let catalogue = Catalogue::builtin().unwrap();
    for (rule, _) in &catalogue {
        assert!(covered.contains(rule.as_str()), "{rule} has no fixture");
    }
}
