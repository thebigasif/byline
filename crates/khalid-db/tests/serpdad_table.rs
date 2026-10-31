//! The `SerpDad` connection on SQLite: one key at a time, the old one
//! handed back to be revoked, drafts newest first and only the last 20,
//! and each site sees only its own (KH-API-6).

use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use khalid_api::keys::KeyId;
use khalid_api::resources::LintCounts;
use khalid_api::serpdad_store::{ArrivedDraft, RECENT_DRAFTS, SerpdadStore};
use khalid_core::{FixedClock, Slug};
use khalid_db::{Database, SerpdadTable};

/// A migrated database with the sites `demo` and `other`, and the
/// connection of each.
#[cfg(test)]
async fn opened() -> (tempfile::TempDir, SerpdadTable, SerpdadTable) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 10, 31, 9, 40, 0).unwrap());
    let demo = db.site(&Slug::try_new("demo").unwrap()).await.unwrap();
    let other = db.site(&Slug::try_new("other").unwrap()).await.unwrap();
    let tables = (
        SerpdadTable::new(db.clone(), demo, clock),
        SerpdadTable::new(db, other, clock),
    );
    (dir, tables.0, tables.1)
}

#[cfg(test)]
fn key(digit: char) -> KeyId {
    KeyId::try_new(&digit.to_string().repeat(16)).unwrap()
}

#[cfg(test)]
fn draft(n: u32, lint: Option<LintCounts>) -> ArrivedDraft {
    ArrivedDraft::new(format!("en.article-{n}"), format!("Article {n}"), lint)
}

#[tokio::test]
async fn the_connection_is_off_until_a_key_is_linked_and_hands_back_the_old_key() {
    let (_dir, demo, other) = opened().await;
    assert_eq!(demo.link().await.unwrap(), None);

    assert_eq!(demo.connect(&key('1')).await.unwrap(), None);
    let link = demo.link().await.unwrap().unwrap();
    assert_eq!(link.key, key('1'));
    assert_eq!(
        link.connected_at,
        Utc.with_ymd_and_hms(2026, 10, 31, 9, 40, 0).unwrap()
    );
    assert_eq!(other.link().await.unwrap(), None, "another site stays off");

    assert_eq!(demo.connect(&key('2')).await.unwrap(), Some(key('1')));
    assert_eq!(demo.link().await.unwrap().unwrap().key, key('2'));

    assert_eq!(demo.disconnect().await.unwrap(), Some(key('2')));
    assert_eq!(demo.link().await.unwrap(), None);
    assert_eq!(demo.disconnect().await.unwrap(), None, "off twice is fine");
}

#[tokio::test]
async fn drafts_come_back_newest_first_with_their_lint_counts() {
    let (_dir, demo, other) = opened().await;
    demo.record(&draft(1, Some(LintCounts::new(0, 2))))
        .await
        .unwrap();
    demo.record(&draft(2, None)).await.unwrap();

    let recent = demo.recent().await.unwrap();
    let posts: Vec<&str> = recent.iter().map(|r| r.draft.post.as_str()).collect();
    assert_eq!(posts, ["en.article-2", "en.article-1"]);
    assert_eq!(recent[0].draft.lint, None, "no linter ran");
    assert_eq!(recent[1].draft, draft(1, Some(LintCounts::new(0, 2))));
    assert_eq!(
        recent[1].received_at,
        Utc.with_ymd_and_hms(2026, 10, 31, 9, 40, 0).unwrap()
    );
    assert_eq!(other.recent().await.unwrap(), []);
}

#[tokio::test]
async fn only_the_last_twenty_drafts_are_kept_and_turning_off_keeps_them() {
    let (_dir, demo, _) = opened().await;
    demo.connect(&key('1')).await.unwrap();
    for n in 1..=RECENT_DRAFTS + 3 {
        demo.record(&draft(n, Some(LintCounts::new(1, 0))))
            .await
            .unwrap();
    }
    demo.disconnect().await.unwrap();

    let recent = demo.recent().await.unwrap();
    assert_eq!(recent.len(), 20);
    assert_eq!(recent[0].draft.post, "en.article-23");
    assert_eq!(recent[19].draft.post, "en.article-4");
}
