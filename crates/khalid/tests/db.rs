//! `khalid db migrate`: the database a site folder keeps in `data/site.db`
//! (ADR 0001, RULES.md BL-CLI-1 to BL-CLI-3).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// The demo site's settings, copied into a fresh folder.
#[cfg(test)]
fn site() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    fs::copy(settings, within(dir.path(), "site.toml")).unwrap();
    fs::create_dir(within(dir.path(), "content")).unwrap();
    dir
}

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// Runs the binary with `args` after `--site <site>`.
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
fn migrate_makes_the_database_and_running_it_again_is_fine() {
    let site = site();
    for _ in 0..2 {
        let output = khalid(site.path(), &["db", "migrate"]);
        assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
        let said = text(&output.stdout);
        assert!(said.contains("data/site.db is up to date."), "{said}");
    }
    assert!(within(site.path(), "data/site.db").is_file());
}

#[test]
fn migrate_with_json_names_the_database() {
    let site = site();
    let output = khalid(site.path(), &["db", "migrate", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let database = json["database"].as_str().unwrap();
    assert!(database.ends_with("data/site.db"), "{json}");
}

#[test]
fn db_needs_a_site_folder() {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["db", "migrate"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("--site"));
}

#[test]
fn a_folder_without_settings_is_refused_before_the_database_is_made() {
    let dir = tempfile::tempdir().unwrap();
    let output = khalid(dir.path(), &["db", "migrate"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("site.toml"));
    assert!(!within(dir.path(), "data").exists());
}

#[test]
fn a_file_where_the_data_folder_belongs_is_explained() {
    let site = site();
    fs::write(within(site.path(), "data"), "not a folder").unwrap();
    let output = khalid(site.path(), &["db", "migrate"]);
    assert_eq!(output.status.code(), Some(1));
    let message = text(&output.stderr);
    let lines: Vec<&str> = message.lines().collect();
    assert_eq!(lines.len(), 3, "{message}");
    assert!(lines[0].contains("data"), "{message}");
    for line in lines {
        assert!(line.ends_with('.'), "{line:?}");
    }
}
