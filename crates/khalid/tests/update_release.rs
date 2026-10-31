//! `khalid update`: the newest release for this system, found through its
//! `SHA256SUMS` and checked against it, in place of the binary, with the
//! old version kept for `--rollback` (plan section 4.1, KH-OPS-2). A local
//! stand-in serves the releases the way GitHub does, and each test runs a
//! copy of the binary in its own folder.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use sha2::{Digest, Sha256};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The name of the binary on this system.
const BINARY: &str = if cfg!(windows) {
    "khalid.exe"
} else {
    "khalid"
};

/// What the release workflow calls this system's file, after the tag.
#[cfg(test)]
fn platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "aarch64") => "macos-aarch64",
        ("windows", "x86_64") => "windows-x86_64.exe",
        _ => "no-release-for-this-system",
    }
}

/// A folder holding a copy of the binary, and the copy's path.
#[cfg(test)]
fn installed() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let mut binary = dir.path().to_path_buf();
    binary.push(BINARY);
    fs::copy(env!("CARGO_BIN_EXE_khalid"), &binary).unwrap();
    (dir, binary)
}

/// A stand-in for the releases whose newest `SHA256SUMS` is `sums`.
#[cfg(test)]
async fn releases(sums: String) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/latest/download/SHA256SUMS"))
        .respond_with(ResponseTemplate::new(200).set_body_string(sums))
        .mount(&server)
        .await;
    server
}

/// A `SHA256SUMS` line for release `tag`'s file for `system`.
#[cfg(test)]
fn line(tag: &str, system: &str) -> String {
    format!("{}  khalid-{tag}-{system}\n", "0".repeat(64))
}

/// A stand-in whose newest release, v99.0.0, has a file for this system
/// holding `bytes`, with `sum` for it in `SHA256SUMS`.
#[cfg(test)]
async fn serving(bytes: &[u8], sum: &str) -> MockServer {
    let file = format!("khalid-v99.0.0-{}", platform());
    let server = releases(format!("{sum}  {file}\n")).await;
    Mock::given(method("GET"))
        .and(path(format!("/download/v99.0.0/{file}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(bytes.to_vec()))
        .mount(&server)
        .await;
    server
}

/// The SHA-256 of `bytes` in lowercase hex.
#[cfg(test)]
fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").unwrap();
            hex
        })
}

/// The file an update keeps the version before it in.
#[cfg(test)]
fn kept(binary: &Path) -> PathBuf {
    let mut name = binary.as_os_str().to_owned();
    name.push(".previous");
    PathBuf::from(name)
}

/// Runs `binary update` against `server`, with `extra` arguments.
#[cfg(test)]
fn update(binary: &Path, server: &MockServer, extra: &[&str]) -> Output {
    let port = server.address().port();
    Command::new(binary)
        .arg("update")
        .args(["--test-hosts", "releases.test=127.0.0.1"])
        .arg("--test-releases")
        .arg(format!("http://releases.test:{port}"))
        .args(extra)
        .output()
        .unwrap()
}

#[cfg(test)]
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[tokio::test]
async fn update_installs_the_newer_release_and_keeps_the_old_version() {
    let (_dir, binary) = installed();
    let running = fs::read(&binary).unwrap();
    let server = serving(b"the new version", &sha(b"the new version")).await;
    let said = text(&update(&binary, &server, &[]).stdout);
    assert!(said.starts_with("Updated Khalid from"), "{said}");
    assert!(said.contains("to 99.0.0"), "{said}");
    assert_eq!(fs::read(&binary).unwrap(), b"the new version");
    assert_eq!(fs::read(kept(&binary)).unwrap(), running);
}

#[tokio::test]
async fn update_with_json_says_which_versions_and_where_the_old_one_is() {
    let (_dir, binary) = installed();
    let server = serving(b"the new version", &sha(b"the new version")).await;
    let output = update(&binary, &server, &["--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["from"], env!("CARGO_PKG_VERSION"), "{json}");
    assert_eq!(json["to"], "99.0.0", "{json}");
    assert_eq!(json["updated"], true, "{json}");
    let named = PathBuf::from(json["kept"].as_str().unwrap());
    assert_eq!(
        fs::canonicalize(named).unwrap(),
        fs::canonicalize(kept(&binary)).unwrap()
    );
}

#[tokio::test]
async fn a_download_that_does_not_match_its_sum_is_refused() {
    let (dir, binary) = installed();
    let running = fs::read(&binary).unwrap();
    let server = serving(b"tampered", &sha(b"the new version")).await;
    let output = update(&binary, &server, &[]);
    assert_eq!(output.status.code(), Some(1));
    let message = text(&output.stderr);
    assert_eq!(message.lines().count(), 3, "{message}");
    assert!(message.contains("SHA256SUMS"), "{message}");
    assert_eq!(fs::read(&binary).unwrap(), running);
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[tokio::test]
async fn the_newest_version_already_running_is_left_alone() {
    let (_dir, binary) = installed();
    let tag = format!("v{}", env!("CARGO_PKG_VERSION"));
    let server = releases(line(&tag, platform())).await;
    let output = update(&binary, &server, &["--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["updated"], false, "{json}");
}

#[tokio::test]
async fn a_release_with_no_file_for_this_system_is_explained() {
    let (_dir, binary) = installed();
    let server = releases(line("v99.0.0", "plan9-mips")).await;
    let output = update(&binary, &server, &[]);
    assert_eq!(output.status.code(), Some(1));
    let message = text(&output.stderr);
    assert_eq!(message.lines().count(), 3, "{message}");
    assert!(message.contains("no file for"), "{message}");
}
