//! `khalid db export`: every table of the site as one CSV and one JSON
//! file each, for spreadsheets and Looker Studio, with no secret in any
//! file (KH-GRO-6, BL-CLI-1 to BL-CLI-3).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// The demo site's settings in a fresh folder, with a webhook stored.
/// Returns the folder and the webhook's secret.
#[cfg(test)]
fn site() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    let settings = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo-site/site.toml"
    );
    fs::copy(settings, within(dir.path(), "site.toml")).unwrap();
    fs::create_dir(within(dir.path(), "content")).unwrap();
    let url = "https://hooks.example.com/alerts";
    let added = khalid(
        dir.path(),
        &[
            "--json",
            "webhooks",
            "add",
            "--event",
            "alert.raised",
            "--url",
            url,
        ],
    );
    assert_eq!(added.status.code(), Some(0), "{}", text(&added.stderr));
    let added: Value = serde_json::from_slice(&added.stdout).unwrap();
    let secret = added["secret"].as_str().unwrap().to_owned();
    (dir, secret)
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
fn export_writes_each_table_as_csv_and_json_without_secrets() {
    let (site, secret) = site();
    let output = khalid(site.path(), &["db", "export"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let said = text(&output.stdout);
    assert!(said.contains("data/export"), "{said}");
    let folder = within(site.path(), "data/export");
    let csv = fs::read_to_string(within(&folder, "webhooks.csv")).unwrap();
    let mut lines = csv.lines();
    assert_eq!(
        lines.next(),
        Some("id,site_id,event,url,created_at,disabled_at")
    );
    assert!(
        lines
            .next()
            .unwrap()
            .contains(",alert.raised,https://hooks.example.com/alerts,")
    );
    let json = fs::read_to_string(within(&folder, "webhooks.json")).unwrap();
    let rows: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(rows[0]["event"], "alert.raised");
    assert_eq!(rows[0]["disabled_at"], Value::Null);
    assert!(rows[0].get("secret").is_none());
    let empty = fs::read_to_string(within(&folder, "redirects.json")).unwrap();
    assert_eq!(empty, "[]\n", "an empty table is an empty list");
    for file in fs::read_dir(&folder).unwrap() {
        let path = file.unwrap().path();
        let written = fs::read_to_string(&path).unwrap();
        assert!(
            !written.contains(&secret),
            "{} holds the secret",
            path.display()
        );
    }
    assert!(!within(&folder, "sessions.csv").exists());
}

#[test]
fn export_with_json_and_out_lists_every_table_and_its_rows() {
    let (site, _) = site();
    let out = within(site.path(), "reports");
    let out_text = out.display().to_string();
    let output = khalid(site.path(), &["--json", "db", "export", "--out", &out_text]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let body: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["folder"], out_text.as_str());
    let tables = body["tables"].as_array().unwrap();
    let webhooks = tables.iter().find(|t| t["name"] == "webhooks").unwrap();
    assert_eq!(webhooks["rows"], 1);
    assert!(tables.iter().any(|t| t["name"] == "not_found_log"));
    assert!(within(&out, "not_found_log.csv").is_file());
}

#[test]
fn export_says_what_to_do_when_the_folder_cannot_be_written() {
    let (site, _) = site();
    let blocked = within(site.path(), "taken");
    fs::write(&blocked, "a file, not a folder").unwrap();
    let blocked = blocked.display().to_string();
    let output = khalid(site.path(), &["db", "export", "--out", &blocked]);
    assert_eq!(output.status.code(), Some(1));
    let said = text(&output.stderr);
    assert!(
        said.contains(&format!("Khalid could not make the folder {blocked}.")),
        "{said}"
    );
}
