//! `khalid keys create` and `revoke`: a key is shown once, stored only as a
//! hash in `data/site.db`, and refused once revoked (RULES.md BL-SEC-3,
//! BL-CLI-1 to BL-CLI-3, ADR 0011).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// The demo site's settings in a fresh folder, with no content yet.
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

/// Creates a key with `args` and `--json`, and returns the JSON it printed.
#[cfg(test)]
fn created(site: &Path, args: &[&str]) -> Value {
    let mut all = vec!["keys", "create", "--json"];
    all.extend_from_slice(args);
    let output = khalid(site, &all);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stdout));
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn each_kind_of_key_is_made_with_its_prefix_and_identity() {
    let site = site();
    for (name, args, prefix) in [
        ("SerpDad", vec!["--scope", "posts:write"], "ki_"),
        ("Web page", vec!["--kind", "content"], "kc_"),
        ("Lena", vec!["--kind", "staff", "--role", "editor"], "ks_"),
    ] {
        let mut args = args;
        args.extend(["--name", name]);
        let json = created(site.path(), &args);
        let key = json["key"].as_str().unwrap();
        let id = json["id"].as_str().unwrap();
        assert!(key.starts_with(prefix), "{json}");
        assert_eq!(key.split('_').nth(1), Some(id), "{json}");
        assert_eq!(json["name"], name);
    }
}

#[test]
fn the_key_is_shown_once_and_never_stored() {
    let site = site();
    let output = khalid(
        site.path(),
        &[
            "keys",
            "create",
            "--name",
            "SerpDad",
            "--scope",
            "posts:write",
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    let key = said
        .split_whitespace()
        .find(|word| word.starts_with("ki_"))
        .unwrap();
    assert_eq!(said.matches(key).count(), 1, "{said}");
    assert!(said.contains("only time"), "{said}");
    let (_, secret) = key.rsplit_once('_').unwrap();
    for name in ["data/site.db", "data/site.db-wal"] {
        let bytes = fs::read(within(site.path(), name)).unwrap_or_default();
        let found = bytes.windows(secret.len()).any(|w| w == secret.as_bytes());
        assert!(!found, "the secret is in {name}");
    }
}

#[test]
fn a_revoked_key_is_revoked_once() {
    let site = site();
    let json = created(site.path(), &["--name", "Zapier", "--scope", "posts:write"]);
    let id = json["id"].as_str().unwrap();
    let first = khalid(site.path(), &["keys", "revoke", id]);
    assert_eq!(first.status.code(), Some(0), "{}", text(&first.stderr));
    assert!(text(&first.stdout).contains(id));
    let again = khalid(site.path(), &["keys", "revoke", id, "--json"]);
    assert_eq!(again.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&again.stdout).unwrap();
    assert!(
        json["error"]["what"].as_str().unwrap().contains(id),
        "{json}"
    );
}

#[test]
fn a_key_without_what_it_may_do_is_refused_before_anything_is_stored() {
    let site = site();
    for args in [
        vec!["keys", "create", "--name", "SerpDad"],
        vec!["keys", "create", "--kind", "staff", "--name", "Lena"],
        vec![
            "keys", "create", "--kind", "content", "--name", "Web", "--role", "editor",
        ],
        vec!["keys", "create", "--name", "X", "--scope", "posts:fly"],
        vec!["keys", "revoke", "not-an-id"],
    ] {
        let output = khalid(site.path(), &args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        let message = text(&output.stderr);
        assert_eq!(message.lines().count(), 3, "{args:?}: {message}");
    }
    assert!(!within(site.path(), "data").exists());
}
