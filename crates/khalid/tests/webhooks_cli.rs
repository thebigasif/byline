//! `khalid webhooks add`, `list` and `remove`: the site's webhooks from the
//! command line, through the same store the Admin API uses, with each
//! secret shown once (task 1.19, KH-API-3, BL-CLI-1 to BL-CLI-3).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

const URL: &str = "https://hooks.example.com/khalid";

/// The demo site's settings in a fresh folder, with no content yet.
#[cfg(test)]
fn site() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    let mut copy = PathBuf::from(dir.path());
    copy.push("site.toml");
    fs::copy(settings, copy).unwrap();
    dir
}

/// Runs `khalid --site <site> webhooks` with `args`; returns the exit
/// code, stdout and stderr.
#[cfg(test)]
fn webhooks(site: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let Output {
        status,
        stdout,
        stderr,
    } = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .arg("webhooks")
        .args(args)
        .output()
        .unwrap();
    let text = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned();
    (status.code(), text(stdout), text(stderr))
}

/// Runs `args` with `--json`, expecting success; returns the JSON.
#[cfg(test)]
fn json(site: &Path, args: &[&str]) -> Value {
    let (code, out, _) = webhooks(site, &[args, &["--json"]].concat());
    assert_eq!(code, Some(0), "{out}");
    serde_json::from_str(&out).unwrap()
}

#[test]
fn a_webhook_is_added_with_its_secret_shown_once_then_listed_without_it() {
    let site = site();
    let added = json(
        site.path(),
        &["add", "--event", "comment.added", "--url", URL],
    );
    assert_eq!(added["id"], 1, "{added}");
    assert_eq!(added["event"], "comment.added");
    assert_eq!(added["url"], URL);
    let secret = added["secret"].as_str().unwrap();
    assert_eq!(secret.len(), 64, "256 random bits as hex");

    let listed = json(site.path(), &["list"]);
    assert_eq!(listed["webhooks"][0]["event"], "comment.added", "{listed}");
    assert!(!listed.to_string().contains(secret), "never listed");
    let (code, shown, _) = webhooks(site.path(), &["list"]);
    assert_eq!(code, Some(0));
    assert!(
        shown.contains("comment.added") && shown.contains(URL),
        "{shown}"
    );

    let args = ["add", "--event", "post.published", "--url", URL];
    let (code, shown, _) = webhooks(site.path(), &args);
    assert_eq!(code, Some(0));
    assert!(shown.contains("only time"), "{shown}");
    assert!(shown.contains("khalid webhooks remove 2"), "{shown}");
}

#[test]
fn a_removed_webhook_is_gone_and_removing_it_again_says_so() {
    let site = site();
    json(
        site.path(),
        &["add", "--event", "post.deleted", "--url", URL],
    );
    let removed = json(site.path(), &["remove", "1"]);
    assert_eq!(removed, json!({"id": 1, "removed": true}));
    assert_eq!(json(site.path(), &["list"]), json!({"webhooks": []}));

    let (code, _, said) = webhooks(site.path(), &["remove", "1"]);
    assert_eq!(code, Some(1));
    let want = "No webhook that is on has the identity 1.";
    assert!(said.contains(want), "{said}");
}

#[test]
fn an_unknown_event_or_a_bad_address_is_refused_before_anything_is_stored() {
    let site = site();
    let args = ["add", "--event", "post.liked", "--url", URL];
    let (code, _, said) = webhooks(site.path(), &args);
    assert_eq!(code, Some(2));
    assert!(said.contains("post.liked is not a webhook event"), "{said}");

    let with_password = "https://me:hunter2@hooks.example.com/";
    let args = ["add", "--event", "post.published", "--url", with_password];
    let (code, _, said) = webhooks(site.path(), &args);
    assert_eq!(code, Some(2));
    assert!(said.contains("not an address Khalid can send to"), "{said}");
    assert!(!said.contains("hunter2"), "the password is never repeated");
    assert_eq!(json(site.path(), &["list"]), json!({"webhooks": []}));
}
