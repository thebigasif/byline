//! `khalid watch` (C6): how fast and how reliably the blog answered at its
//! public address, from the looks `khalid serve` kept.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use chrono::Duration;
use khalid_alerts::{Answer, Probe};
use khalid_core::{Clock, Slug, SystemClock};
use khalid_db::{Database, WatchTable};
use serde_json::Value;
use tempfile::TempDir;

const HOME: &str = "https://www.example.com/blog/";

/// A copy of the demo site, with `looks` kept as `(minutes ago, answer,
/// millis)`.
#[cfg(test)]
async fn site_with(looks: &[(i64, Answer, u32)]) -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    let mut path = temp.path().to_path_buf();
    path.push("data");
    fs::create_dir_all(&path).unwrap();
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let table = WatchTable::new(db.clone(), site);
    let now = SystemClock.now();
    for &(ago, answer, millis) in looks {
        let at = now - Duration::minutes(ago);
        table
            .record(HOME, &Probe { at, answer, millis })
            .await
            .unwrap();
    }
    db.close().await;
    temp
}

/// Runs `khalid watch` on `site` with `extra` arguments, and returns what
/// it printed.
#[cfg(test)]
fn watch(site: &TempDir, extra: &[&str]) -> String {
    let ran = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.path().to_str().unwrap(), "watch"])
        .args(extra)
        .output()
        .unwrap();
    assert!(ran.status.success(), "{ran:?}");
    String::from_utf8(ran.stdout).unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn the_report_counts_the_last_day_and_the_last_30_days() {
    let site = site_with(&[
        (3 * 24 * 60, Answer::Status(200), 300),
        (10, Answer::Status(200), 100),
        (5, Answer::Unreached, 10_000),
    ])
    .await;
    let json: Value = serde_json::from_str(&watch(&site, &["--json"])).unwrap();
    let page = &json["pages"][0];
    assert_eq!(page["page"], HOME);
    assert_eq!(page["last_day"]["looks"], 2);
    assert_eq!(page["last_day"]["answered"], 1);
    assert_eq!(page["last_day"]["middle_ms"], 100);
    assert_eq!(page["last_30_days"]["looks"], 3);
    assert_eq!(page["last_30_days"]["slowest_ms"], 300);
    assert_eq!(
        page["last_30_days"]["longest_gap_minutes"],
        3 * 24 * 60 - 10
    );
    assert_eq!(page["last_look"]["status"], Value::Null);

    let text = watch(&site, &[]);
    assert!(text.contains(HOME), "{text}");
    assert!(
        text.contains("Last 24 hours: 2 looks, 50.0% answered, middle 100 ms, slowest 100 ms"),
        "{text}"
    );
    assert!(text.contains("got no answer"), "{text}");
}

#[tokio::test(flavor = "multi_thread")]
async fn with_no_looks_it_says_when_the_watch_looks() {
    let site = site_with(&[]).await;
    let text = watch(&site, &[]);
    assert!(
        text.starts_with("The watch has not looked at the blog yet."),
        "{text}"
    );
    let json: Value = serde_json::from_str(&watch(&site, &["--json"])).unwrap();
    assert_eq!(json["pages"], serde_json::json!([]));
}
