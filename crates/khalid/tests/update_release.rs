//! `khalid update`: the newest release for this system, found through its
//! `SHA256SUMS` (plan section 4.1, KH-OPS-2). A local stand-in serves the
//! releases the way GitHub does, and each test runs a copy of the binary in
//! its own folder.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
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
async fn a_newer_release_is_named_with_where_to_download_it() {
    let (dir, binary) = installed();
    let running = fs::read(&binary).unwrap();
    let sums = [line("v99.0.0", "plan9-mips"), line("v99.0.0", platform())];
    let server = releases(sums.concat()).await;
    let said = text(&update(&binary, &server, &[]).stdout);
    let file = format!("download/v99.0.0/khalid-v99.0.0-{}", platform());
    assert!(said.starts_with("Khalid 99.0.0 is out"), "{said}");
    assert!(said.contains(&file), "{said}");
    let output = update(&binary, &server, &["--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["from"], env!("CARGO_PKG_VERSION"), "{json}");
    assert_eq!(json["to"], "99.0.0", "{json}");
    assert_eq!(json["updated"], false, "{json}");
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
