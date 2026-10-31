//! `khalid plan` suggests a day for each ready draft, the pillar of a topic
//! group before its members, keeps scheduled days, lists the drafts the
//! linter blocks, and changes nothing (GEO task M4, KH-OPS-2).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use chrono::Days;
use khalid_api::schedule_store::{Schedule, ScheduleStore};
use khalid_core::{Clock, Slug, SystemClock};
use khalid_db::{Database, ScheduleTable};
use serde_json::Value;
use tempfile::TempDir;

/// A draft the gate lets through, in topic group `anchors` as `role`.
fn good(slug: &str, role: &str) -> String {
    format!(
        "---\ntitle: \"{slug}\"\nslug: {slug}\nauthor: asif\n\
         cluster:\n  name: anchors\n  role: {role}\n---\n\
         Anchor text is the visible words of a link.\n\n\
         Read [why internal links matter](/blog/why-internal-links-matter/) first.\n"
    )
}

/// A draft the gate refuses.
const WEAK: &str = "---\ntitle: \"A weak post\"\nslug: a-weak-post\n---\nShort.\n";

/// `rest` inside `dir`.
fn at(dir: &Path, rest: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(rest);
    path
}

/// A copy of the demo site with a pillar, a member that sorts before it,
/// a weak draft, and a draft scheduled ten days from now.
#[cfg(test)]
async fn site() -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    let drafts = at(temp.path(), "drafts/en");
    fs::create_dir_all(&drafts).unwrap();
    for (slug, text) in [
        ("a-member", good("a-member", "member")),
        ("z-pillar", good("z-pillar", "pillar")),
        ("set-later", good("set-later", "member")),
        ("a-weak-post", WEAK.to_owned()),
    ] {
        fs::write(at(&drafts, &format!("{slug}.md")), text).unwrap();
    }
    fs::create_dir_all(at(temp.path(), "data")).unwrap();
    let db = Database::open(&at(temp.path(), "data/site.db"))
        .await
        .unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let later = SystemClock.now().checked_add_days(Days::new(10)).unwrap();
    ScheduleTable::new(db.clone(), site)
        .set("en.set-later", Schedule::default().publishing_at(later))
        .await
        .unwrap();
    db.close().await;
    temp
}

/// Runs `khalid plan` with `args` on `site`.
#[cfg(test)]
fn plan(site: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "plan"])
        .args(args)
        .output()
        .unwrap()
}

/// The JSON a successful run printed.
#[cfg(test)]
fn json(output: &Output) -> Value {
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test]
async fn the_pillar_gets_the_first_day_and_the_weak_draft_waits() {
    let site = site().await;
    let before = fs::read_dir(at(site.path(), "drafts/en")).unwrap().count();
    let body = json(&plan(site.path(), &["--per-week", "7", "--json"]));
    assert_eq!(body["per_week"], 7);
    assert_eq!(body["pace_from"], "asked");
    let calendar: Vec<(&str, &str)> = body["calendar"]
        .as_array()
        .unwrap()
        .iter()
        .map(|slot| (slot["id"].as_str().unwrap(), slot["kind"].as_str().unwrap()))
        .collect();
    assert_eq!(
        calendar,
        [
            ("en.z-pillar", "suggested"),
            ("en.a-member", "suggested"),
            ("en.set-later", "scheduled"),
        ]
    );
    let tomorrow = SystemClock
        .now()
        .date_naive()
        .checked_add_days(Days::new(1));
    assert_eq!(
        body["calendar"][0]["day"].as_str(),
        tomorrow.map(|day| day.to_string()).as_deref()
    );
    assert_eq!(body["waiting"][0]["id"], "en.a-weak-post");
    let after = fs::read_dir(at(site.path(), "drafts/en")).unwrap().count();
    assert_eq!(before, after, "the planner changed the drafts");
}

#[tokio::test]
async fn the_plan_reads_as_sentences_and_says_nothing_changed() {
    let site = site().await;
    let output = plan(site.path(), &[]);
    assert!(output.status.success(), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("a week, the site's pace over the last twelve weeks."),
        "{text}"
    );
    assert!(text.contains("suggested  en.z-pillar"), "{text}");
    assert!(
        text.contains("en.a-weak-post  A weak post: the linter blocks it."),
        "{text}"
    );
    assert!(text.contains("Nothing was changed."), "{text}");
}

#[test]
fn a_pace_outside_one_to_fourteen_is_refused() {
    let output = plan(Path::new("."), &["--per-week", "15"]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("from 1 to 14"), "{error}");
}
