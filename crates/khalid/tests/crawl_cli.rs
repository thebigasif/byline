//! `khalid crawl health` on the demo site, with a
//! visitor log written beforehand (GEO C3). With the log off,
//! each says so.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use khalid_analytics::{
    BotCheck, Crawler, ShortAddress, Visit, Visitor, VisitorKey, VisitorLogStore,
};
use khalid_core::{Clock, Slug, SystemClock};
use khalid_db::{Database, VisitorLogTable};
use serde_json::Value;
use tempfile::TempDir;

const POST: &str = "/blog/planning-a-topic-cluster/";

/// Every `khalid crawl` report.
const REPORTS: &[&str] = &["health"];

#[cfg(test)]
fn visit(at: DateTime<Utc>, path: &str, status: u16, (name, check): (&str, BotCheck)) -> Visit {
    let address: IpAddr = "66.249.66.1".parse().unwrap();
    Visit {
        at,
        path: path.to_owned(),
        status,
        address: ShortAddress::of(address),
        code: VisitorKey::from_bytes([6; 32]).code(address, None),
        visitor: Visitor::Bot(Crawler::named(name).unwrap(), check),
    }
}

/// A copy of the demo site with `extra` settings and a visitor log: the
/// first crawl the log saw on 11 October, Googlebot on the 12 October
/// post six hours after it went live, and today's visits.
#[cfg(test)]
async fn demo_site(extra: &str) -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    let mut settings = temp.path().to_path_buf();
    settings.push("site.toml");
    let mut file = OpenOptions::new().append(true).open(&settings).unwrap();
    writeln!(file, "\n{extra}").unwrap();
    let mut path = temp.path().to_path_buf();
    path.push("data");
    fs::create_dir_all(&path).unwrap();
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let day = |d, h| Utc.with_ymd_and_hms(2026, 10, d, h, 0, 0).unwrap();
    let now = SystemClock.now() - TimeDelta::hours(1);
    let google = ("Googlebot", BotCheck::Listed);
    let visits = [
        visit(day(11, 0), "/blog/", 200, ("Bingbot", BotCheck::Listed)),
        visit(day(12, 6), POST, 200, google),
        visit(now, POST, 200, google),
        visit(now, "/blog/gone/", 404, google),
        visit(now, "/blog/gone/", 404, ("GPTBot", BotCheck::Listed)),
        visit(
            now,
            "/wp-login.php",
            404,
            ("Googlebot", BotCheck::NotListed),
        ),
    ];
    let log = VisitorLogTable::new(db.clone(), site);
    log.save(&visits, DateTime::<Utc>::MIN_UTC).await.unwrap();
    db.close().await;
    temp
}

/// Runs `khalid crawl` with `args` on `site`, and returns the exit code
/// and standard output.
#[cfg(test)]
fn crawl(site: &Path, args: &[&str]) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "crawl"])
        .args(args)
        .output()
        .unwrap();
    let out = String::from_utf8(output.stdout).unwrap();
    (output.status.code().unwrap(), out)
}

#[tokio::test]
async fn health_counts_googlebot_and_sets_pretenders_apart() {
    let site = demo_site("[visitor_log]\non = true").await;
    let (code, out) = crawl(site.path(), &["health", "--json"]);
    assert_eq!(code, 0, "{out}");
    let json: Value = serde_json::from_str(&out).unwrap();
    let crawlers = json["crawlers"].as_array().unwrap();
    let names: Vec<&str> = crawlers
        .iter()
        .map(|one| one["crawler"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Bingbot", "Googlebot", "GPTBot"], "{json}");
    let google = &crawlers[1];
    assert_eq!(google["listed"]["ok"], 1);
    assert_eq!(google["listed"]["missing"], 1);
    assert_eq!(google["pretending"], 1);
    let (_, text) = crawl(site.path(), &["health"]);
    assert!(
        text.contains("  1 visit from other addresses using its name, not counted"),
        "{text}"
    );
}

#[tokio::test]
async fn every_report_says_the_visitor_log_is_off_while_it_is() {
    let site = demo_site("").await;
    for what in REPORTS {
        let (code, out) = crawl(site.path(), &[what, "--json"]);
        assert_eq!(code, 1, "{out}");
        let json: Value = serde_json::from_str(&out).unwrap();
        let said = json["error"]["what"].as_str().unwrap();
        assert!(said.starts_with("The visitor log is off"), "{said}");
    }
}
