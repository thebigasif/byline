//! `khalid audit --server` on a running `khalid serve`: the crawl asks the
//! server on this machine for the site's own addresses (KH-LNT-5).

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::Value;
use tempfile::TempDir;

/// A running `khalid serve`, stopped when dropped.
struct Served {
    child: Child,
    address: String,
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
fn demo_copy() -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    temp
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == "data" {
            continue;
        }
        let mut target = into.to_path_buf();
        target.push(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Starts `khalid serve` on a free port and returns its origin, such as
/// `http://127.0.0.1:41234`.
#[cfg(test)]
fn serve(site: &Path) -> Served {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "serve", "--listen"])
        .args(["127.0.0.1:0", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    let stdout = child.stdout.take().unwrap();
    BufReader::new(stdout).read_line(&mut line).unwrap();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line}"));
    let address = json["address"].as_str().unwrap();
    let address = address.trim_end_matches("/blog/").to_owned();
    Served { child, address }
}

#[test]
fn a_running_server_is_crawled_at_the_sites_own_addresses() {
    let site = demo_copy();
    let served = serve(site.path());
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.path().to_str().unwrap(), "audit", "--json"])
        .args(["--server", &served.address])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let body: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| panic!("{stdout}"));
    let issues = body["issues"].as_array().unwrap();
    assert_eq!(body["errors"], 0, "{stdout}");
    assert_eq!(body["score"], 100, "{stdout}");
    assert!(body["pages"].as_u64().unwrap() > 10, "{stdout}");
    // Every address is the site's own, so no page looks non-canonical.
    assert!(
        issues.iter().all(|issue| issue["url"]
            .as_str()
            .unwrap()
            .starts_with("https://www.example.com/blog/")
            && issue["check"] != "non-canonical"),
        "{stdout}"
    );
    // A plain HTTP server sends no HSTS header: a notice on each page.
    assert!(
        issues.iter().any(|issue| issue["check"] == "hsts-missing"),
        "{stdout}"
    );
    assert_eq!(output.status.code(), Some(0), "{stdout}");
}
