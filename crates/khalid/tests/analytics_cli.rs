//! `khalid analytics uncited`: the pages AI bots read that no AI assistant
//! sent a reader to, read from the site's database (GEO M2, KH-GRO-4).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use khalid_analytics::{AiTool, AnalyticsStore, Crawler, ReferrerGroup, Tally};
use khalid_core::{Clock, Slug, SystemClock};
use khalid_db::{AnalyticsTable, Database};
use serde_json::Value;

const GPTBOT: &str = "Mozilla/5.0 (compatible; GPTBot/1.2; +https://openai.com/gptbot)";
const CLAUDEBOT: &str = "Mozilla/5.0 (compatible; ClaudeBot/1.0; +claudebot@anthropic.com)";
const QUIET: &str = "/blog/why-internal-links-matter/";
const CITED: &str = "/blog/hello-world/";

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

/// Stores today's counts in the site's database, as `khalid serve` would:
/// AI bots read both pages, and one reader came to `CITED` from an AI assistant.
#[cfg(test)]
async fn counted(site: &Path) {
    let mut file = PathBuf::from(site);
    file.push("data");
    fs::create_dir_all(&file).unwrap();
    file.push("site.db");
    let db = Database::open(&file).await.unwrap();
    db.migrate().await.unwrap();
    let id = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let now = SystemClock.now();
    let gptbot = Crawler::recognise(Some(GPTBOT)).unwrap();
    let claudebot = Crawler::recognise(Some(CLAUDEBOT)).unwrap();
    let mut tally = Tally::default();
    for _ in 0..3 {
        tally.crawl(now, QUIET, gptbot, 200);
    }
    tally.crawl(now, QUIET, claudebot, 200);
    tally.crawl(now, CITED, gptbot, 200);
    tally.crawl(now, "/robots.txt", gptbot, 200);
    tally.view(now, CITED, ReferrerGroup::Ai(AiTool::Perplexity));
    tally.view(now, QUIET, ReferrerGroup::Search);
    AnalyticsTable::new(db.clone(), id)
        .save(&tally)
        .await
        .unwrap();
    db.close().await;
}

/// Runs `khalid --site <site> analytics uncited` with `args`; returns the
/// exit code, stdout and stderr.
#[cfg(test)]
fn uncited(site: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let Output {
        status,
        stdout,
        stderr,
    } = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(site)
        .args(["analytics", "uncited"])
        .args(args)
        .output()
        .unwrap();
    let text = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned();
    (status.code(), text(stdout), text(stderr))
}

#[tokio::test]
async fn a_page_ai_bots_read_with_no_ai_reader_is_listed_as_json() {
    let site = site();
    counted(site.path()).await;
    let (code, out, err) = uncited(site.path(), &["--json"]);
    assert_eq!(code, Some(0), "{out}{err}");
    let json: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["days"], 30, "{json}");
    let pages = json["pages"].as_array().unwrap();
    assert_eq!(pages.len(), 1, "{json}");
    assert_eq!(pages[0]["path"], QUIET);
    assert_eq!(pages[0]["reads"], 4);
    assert_eq!(pages[0]["bots"], serde_json::json!(["ClaudeBot", "GPTBot"]));
    assert!(pages[0]["last_read"].is_string(), "{json}");
}

#[tokio::test]
async fn a_person_reads_the_page_and_what_to_do() {
    let site = site();
    counted(site.path()).await;
    let (code, out, _) = uncited(site.path(), &["--days", "7"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains(QUIET), "{out}");
    assert!(out.contains("4 reads by ClaudeBot, GPTBot"), "{out}");
    assert!(!out.contains(CITED), "{out}");
    assert!(out.contains("last 7 days"), "{out}");
    assert!(!out.contains('\u{2014}'), "no em dash: {out}");
}

#[test]
fn a_site_with_no_reads_says_so() {
    let site = site();
    let (code, out, _) = uncited(site.path(), &[]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("No page"), "{out}");
    let (code, out, _) = uncited(site.path(), &["--json"]);
    assert_eq!(code, Some(0), "{out}");
    let json: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["pages"], serde_json::json!([]), "{json}");
}

#[test]
fn a_period_the_crawl_log_does_not_keep_is_refused() {
    let site = site();
    for days in ["0", "91"] {
        let (code, out, err) = uncited(site.path(), &["--days", days, "--json"]);
        assert_eq!(code, Some(2), "{days}: {out}{err}");
    }
}
