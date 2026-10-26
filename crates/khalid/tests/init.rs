//! `khalid init`: a site folder from four answers, or from flags when there is
//! no terminal to ask in (RULES.md BL-CLI-1 to BL-CLI-4, PRD KH-INS-2).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use khalid_core::{Config, Environment, Mode};
use serde_json::Value;

/// Runs the binary with stdin closed, which is not a terminal.
#[cfg(test)]
fn khalid(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Checks a message the way BL-CLI-3 asks: three short sentences.
fn assert_what_why_fix(stderr: &str) {
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 3, "want what, why, fix:\n{stderr}");
    for line in lines {
        assert!(line.ends_with('.'), "{line:?}");
        assert!(line.split_whitespace().count() < 25, "{line:?}");
        assert!(!line.contains('\u{2014}'), "{line:?}");
    }
}

fn at(base: &Path, rel: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    path.push(rel);
    path
}

#[cfg(test)]
fn path(folder: &Path) -> &str {
    folder.to_str().unwrap()
}

#[test]
fn init_with_flags_writes_a_site_that_loads() {
    let temp = tempfile::tempdir().unwrap();
    let site = at(temp.path(), "blog");
    let output = khalid(&[
        "init",
        path(&site),
        "--address",
        "https://www.example.com",
        "--mount",
        "/blog/",
        "--languages",
        "de,en",
        "--mode",
        "static",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains("khalid --site"));

    let config = Config::load(&site, &Environment::empty()).unwrap();
    assert_eq!(config.address().as_str(), "https://www.example.com/");
    assert_eq!(config.name(), "www.example.com");
    assert_eq!(config.mount(), "/blog/");
    assert_eq!(config.mode(), Mode::Static);
    assert_eq!(config.default_language().as_str(), "de");
    assert_eq!(config.languages().len(), 2);
    for folder in ["theme/templates", "content/de/posts", "content/en/posts"] {
        assert!(at(&site, folder).is_dir(), "{folder} is missing");
    }

    let checked = khalid(&["--site", path(&site)]);
    assert_eq!(checked.status.code(), Some(0), "{}", text(&checked.stderr));
}

#[test]
fn init_with_json_takes_the_defaults_and_lists_what_it_wrote() {
    let temp = tempfile::tempdir().unwrap();
    let output = khalid(&[
        "init",
        path(temp.path()),
        "--address",
        "https://www.example.com",
        "--json",
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(text(&output.stderr), "");
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let created = [
        "site.toml",
        "routes.toml",
        "theme/templates/",
        "content/en/posts/",
    ];
    assert_eq!(
        json,
        serde_json::json!({ "site": path(temp.path()), "created": created })
    );
    let config = Config::load(temp.path(), &Environment::empty()).unwrap();
    assert_eq!(config.mount(), "/");
    assert_eq!(config.mode(), Mode::Server);
    assert_eq!(config.languages().len(), 1);
    assert_eq!(config.default_language().as_str(), "en");
}

#[test]
fn init_needs_the_address_flag() {
    let temp = tempfile::tempdir().unwrap();
    let output = khalid(&["init", path(temp.path())]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = text(&output.stderr);
    assert_what_why_fix(&stderr);
    assert!(stderr.contains("--address"), "{stderr}");
    assert!(!at(temp.path(), "site.toml").exists());
}
