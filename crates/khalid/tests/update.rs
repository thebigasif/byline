//! `khalid update --rollback`: the binary goes back to the version kept
//! beside it by the last update (plan section 4.1, KH-OPS-2). Each test
//! runs a copy of the binary in its own folder, so the copy, not the test's
//! binary, is swapped.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// The name of the binary on this system.
const BINARY: &str = if cfg!(windows) {
    "khalid.exe"
} else {
    "khalid"
};

/// A folder holding a copy of the binary, and the copy's path.
#[cfg(test)]
fn installed() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let binary = within(dir.path(), BINARY);
    fs::copy(env!("CARGO_BIN_EXE_khalid"), &binary).unwrap();
    (dir, binary)
}

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// The file an update keeps the version before it in.
#[cfg(test)]
fn kept(binary: &Path) -> PathBuf {
    let mut name = binary.as_os_str().to_owned();
    name.push(".previous");
    PathBuf::from(name)
}

#[cfg(test)]
fn run(binary: &Path, args: &[&str]) -> Output {
    Command::new(binary).args(args).output().unwrap()
}

#[cfg(test)]
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn rollback_swaps_the_binary_with_the_version_kept_before_the_last_update() {
    let (_dir, binary) = installed();
    let running = fs::read(&binary).unwrap();
    fs::write(kept(&binary), b"the version before").unwrap();
    let output = run(&binary, &["update", "--rollback"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.starts_with("Khalid went back"), "{said}");
    assert_eq!(fs::read(&binary).unwrap(), b"the version before");
    // The version that ran is kept, so a second rollback undoes the first.
    assert_eq!(fs::read(kept(&binary)).unwrap(), running);
}

#[test]
fn rollback_with_json_names_the_binary_and_the_kept_version() {
    let (_dir, binary) = installed();
    fs::write(kept(&binary), b"the version before").unwrap();
    let output = run(&binary, &["update", "--rollback", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    let named = |field: &str| PathBuf::from(json[field].as_str().unwrap());
    assert_eq!(
        fs::canonicalize(named("binary")).unwrap(),
        fs::canonicalize(&binary).unwrap(),
        "{json}"
    );
    assert_eq!(
        fs::canonicalize(named("kept")).unwrap(),
        fs::canonicalize(kept(&binary)).unwrap(),
        "{json}"
    );
}

#[test]
fn rollback_with_no_kept_version_changes_nothing_and_says_why() {
    let (dir, binary) = installed();
    let running = fs::read(&binary).unwrap();
    let output = run(&binary, &["update", "--rollback"]);
    assert_eq!(output.status.code(), Some(1));
    let message = text(&output.stderr);
    let lines: Vec<&str> = message.lines().collect();
    assert_eq!(lines.len(), 3, "{message}");
    assert!(lines[0].contains("no earlier version"), "{message}");
    for line in lines {
        assert!(line.ends_with('.'), "{line:?}");
    }
    assert_eq!(fs::read(&binary).unwrap(), running);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}
