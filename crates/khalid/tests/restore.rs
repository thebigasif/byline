//! `khalid restore`: a backup made by `khalid backup` becomes a working site
//! in a new folder, database included, and nothing else is ever written
//! over (KH-OPS-2).

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::{Server, demo_copy, new_key, start};
use serde_json::Value;

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// Runs the binary with `args`.
#[cfg(test)]
fn khalid(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(args)
        .output()
        .unwrap()
}

#[cfg(test)]
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Backs up `site` to `blog.tar.gz` in `out` and returns the archive.
#[cfg(test)]
fn backup(site: &Path, out: &Path) -> PathBuf {
    let archive = within(out, "blog.tar.gz");
    let made = khalid(&[
        "--site",
        site.to_str().unwrap(),
        "backup",
        "--out",
        archive.to_str().unwrap(),
    ]);
    assert_eq!(made.status.code(), Some(0), "{}", text(&made.stderr));
    assert!(text(&made.stdout).contains("khalid restore"));
    archive
}

/// The backup of a served site, restored into a new folder, serves the same
/// blog and still knows the API key made before the backup.
#[test]
fn a_restored_backup_serves_the_site_with_its_database() {
    let site = demo_copy();
    let key = new_key(site.path(), &["--name", "Owner", "--scope", "admin:all"]);
    let out = tempfile::tempdir().unwrap();
    let archive = backup(site.path(), out.path());
    let restored = within(out.path(), "restored");
    let made = khalid(&[
        "restore",
        archive.to_str().unwrap(),
        restored.to_str().unwrap(),
    ]);
    assert_eq!(made.status.code(), Some(0), "{}", text(&made.stderr));
    let said = text(&made.stdout);
    assert!(said.starts_with("Restored "), "{said}");
    assert!(said.contains("serve"), "{said}");
    assert!(within(&restored, "site.toml").is_file());
    assert!(within(&restored, "data/site.db").is_file());
    let server: Server = start(&restored, &key);
    assert!(
        server.answers("/blog/", 200),
        "the restored site is not served"
    );
    let (status, trash) = server.admin("GET", "/api/admin/v1/trash/", None);
    assert_eq!(
        status, 200,
        "the key from before the backup is gone: {trash}"
    );
}

#[test]
fn restore_with_json_names_the_folder_and_counts_the_files() {
    let site = demo_copy();
    let out = tempfile::tempdir().unwrap();
    let archive = backup(site.path(), out.path());
    let restored = within(out.path(), "restored");
    let made = khalid(&[
        "--json",
        "restore",
        archive.to_str().unwrap(),
        restored.to_str().unwrap(),
    ]);
    assert_eq!(made.status.code(), Some(0), "{}", text(&made.stdout));
    let json: Value = serde_json::from_slice(&made.stdout).unwrap();
    assert!(json["folder"].as_str().unwrap().ends_with("restored"));
    assert!(json["files"].as_u64().unwrap() > 1, "{json}");
    assert_eq!(json["database"], false, "{json}");
}

/// Restoring never writes over a folder that holds anything, such as the
/// live site, so its database and files stay as they were.
#[test]
fn restore_never_writes_into_a_folder_that_holds_files() {
    let site = demo_copy();
    let out = tempfile::tempdir().unwrap();
    let archive = backup(site.path(), out.path());
    let settings = fs::read_to_string(within(site.path(), "site.toml")).unwrap();
    let made = khalid(&[
        "restore",
        archive.to_str().unwrap(),
        site.path().to_str().unwrap(),
    ]);
    assert_eq!(made.status.code(), Some(1));
    let message = text(&made.stderr);
    assert_eq!(message.lines().count(), 3, "{message}");
    assert!(message.contains("already holds files"), "{message}");
    let kept = fs::read_to_string(within(site.path(), "site.toml")).unwrap();
    assert_eq!(kept, settings);
}

#[test]
fn restore_fills_an_empty_folder() {
    let site = demo_copy();
    let out = tempfile::tempdir().unwrap();
    let archive = backup(site.path(), out.path());
    let restored = within(out.path(), "restored");
    fs::create_dir(&restored).unwrap();
    let made = khalid(&[
        "restore",
        archive.to_str().unwrap(),
        restored.to_str().unwrap(),
    ]);
    assert_eq!(made.status.code(), Some(0), "{}", text(&made.stderr));
    assert!(within(&restored, "site.toml").is_file());
}
