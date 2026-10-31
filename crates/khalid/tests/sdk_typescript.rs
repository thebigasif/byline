//! The TypeScript SDK in `sdk/typescript/` lists the demo posts from a
//! running `khalid serve` (task 1.20, KH-API-4). Node runs the SDK's own
//! test, `test/demo-posts.test.js`, against the server.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::Value;

/// A running `khalid serve`, stopped when dropped.
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A path inside the repository.
#[cfg(test)]
fn repository(path: &str) -> PathBuf {
    let mut at = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    at.push("../..");
    at.push(path);
    at
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let mut target = into.to_path_buf();
        target.push(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Runs `khalid` on `site` with `args` and reads the JSON it prints.
#[cfg(test)]
fn khalid_json(site: &Path, args: &[&str]) -> (Child, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap()])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    let stdout = child.stdout.take().unwrap();
    BufReader::new(stdout).read_line(&mut line).unwrap();
    let json = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line}"));
    (child, json)
}

#[test]
fn the_typescript_sdk_lists_the_demo_posts_from_khalid_serve() {
    let site = tempfile::tempdir().unwrap();
    copy_dir(&repository("fixtures/demo-site"), site.path());
    let create = [
        "keys", "create", "--json", "--kind", "content", "--name", "SDK",
    ];
    let (mut keys, made) = khalid_json(site.path(), &create);
    assert!(keys.wait().unwrap().success(), "khalid keys create failed");
    let key = made["key"].as_str().unwrap().to_owned();
    let serve = ["serve", "--listen", "127.0.0.1:0", "--json"];
    let (child, started) = khalid_json(site.path(), &serve);
    let _server = Server(child);
    let address = started["address"].as_str().unwrap();
    let base = address.trim_end_matches("/blog/");

    let test = repository("sdk/typescript/test/demo-posts.test.js");
    let output = Command::new("node")
        .args(["--test", test.to_str().unwrap()])
        .env("KHALID_URL", base)
        .env("KHALID_CONTENT_KEY", &key)
        .output()
        .unwrap_or_else(|error| {
            panic!("Node did not start: {error}. Install Node 18 or later to run the SDK test.")
        });
    let shown = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{shown}");
    assert!(shown.contains("# pass 3"), "{shown}");
}
