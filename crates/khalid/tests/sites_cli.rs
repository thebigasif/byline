//! `khalid sites add`, `admin` and `list`: a folder becomes a sites folder
//! with its first domain, each site gets its own site folder, and the
//! list and its super admins live in `sites.toml` (plan 4.4, KH-EDT-7).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{Value, json};

/// Runs the binary with `args` after `--site <root>`.
#[cfg(test)]
fn khalid(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(root)
        .args(args)
        .output()
        .unwrap()
}

#[cfg(test)]
fn within(dir: &Path, parts: &[&str]) -> PathBuf {
    let mut path = dir.to_path_buf();
    for part in parts {
        path.push(part);
    }
    path
}

#[cfg(test)]
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn a_folder_gains_sites_each_in_its_own_folder_and_lists_them() {
    let root = tempfile::tempdir().unwrap();
    for domain in ["example.com", "News.Example.com"] {
        let added = khalid(root.path(), &["sites", "add", domain]);
        assert_eq!(added.status.code(), Some(0), "{}", text(&added.stderr));
    }
    let admin = khalid(root.path(), &["sites", "admin", "owner@example.com"]);
    assert_eq!(admin.status.code(), Some(0), "{}", text(&admin.stderr));
    assert!(text(&admin.stdout).contains("example.com"));

    let listed = khalid(root.path(), &["sites", "list", "--json"]);
    assert_eq!(listed.status.code(), Some(0), "{}", text(&listed.stderr));
    let json: Value = serde_json::from_slice(&listed.stdout).unwrap();
    let folder = |domain| {
        within(root.path(), &["sites", domain])
            .display()
            .to_string()
    };
    let want = json!({ "sites": [
        { "domain": "example.com", "folder": folder("example.com"), "home": true },
        { "domain": "news.example.com", "folder": folder("news.example.com"), "home": false },
    ]});
    assert_eq!(json, want);

    let settings = within(root.path(), &["sites", "news.example.com", "site.toml"]);
    let settings = fs::read_to_string(settings).unwrap();
    assert!(
        settings.contains("address = \"https://news.example.com/\""),
        "{settings}"
    );
    let list = fs::read_to_string(within(root.path(), &["sites.toml"])).unwrap();
    assert!(
        list.contains("super_admins = [\"owner@example.com\"]"),
        "{list}"
    );

    // The new folder is a whole site: its settings pass the usual check.
    let news = within(root.path(), &["sites", "news.example.com"]);
    let post = khalid(&news, &["new", "First news"]);
    assert_eq!(post.status.code(), Some(0), "{}", text(&post.stderr));
}

#[test]
fn a_taken_domain_a_bad_one_and_a_plain_folder_are_refused_with_the_reason() {
    let root = tempfile::tempdir().unwrap();
    let added = khalid(root.path(), &["sites", "add", "example.com"]);
    assert_eq!(added.status.code(), Some(0), "{}", text(&added.stderr));

    let again = khalid(root.path(), &["sites", "add", "EXAMPLE.com"]);
    assert_eq!(again.status.code(), Some(1));
    assert!(text(&again.stderr).contains("already answers on example.com"));

    for bad in ["../etc", "a b.com", "a..b.com"] {
        let refused = khalid(root.path(), &["sites", "add", bad]);
        assert_eq!(
            refused.status.code(),
            Some(2),
            "{bad}: {}",
            text(&refused.stderr)
        );
    }
    assert!(!within(root.path(), &["etc"]).exists());

    let plain = tempfile::tempdir().unwrap();
    for args in [
        &["sites", "list"][..],
        &["sites", "admin", "owner@example.com"],
    ] {
        let refused = khalid(plain.path(), args);
        assert_eq!(refused.status.code(), Some(1), "{args:?}");
        assert!(
            text(&refused.stderr).contains("serves one site, not many"),
            "{args:?}"
        );
        assert!(!within(plain.path(), &["sites.toml"]).exists());
    }
}
