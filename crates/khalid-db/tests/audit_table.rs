//! The audit log on SQLite: each change is kept with who made it and when,
//! newest first, paged by the last row seen, and each site sees only its
//! own (KH-EDT-6).

use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeZone, Utc};
use khalid_api::audit_log::{Action, AuditLog, Change};
use khalid_core::Slug;
use khalid_db::{AuditTable, Database};

#[cfg(test)]
fn at(minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 3, 9, minute, 0).unwrap()
}

#[cfg(test)]
fn change(minute: u32, action: Action, path: &str) -> Change {
    Change::new(
        at(minute),
        "0123456789abcdef",
        action,
        "/api/admin/v1/posts/{id}/",
        path,
    )
}

/// The audit log of site `handle` in the database at `path`.
#[cfg(test)]
async fn table(path: &Path, handle: &str) -> (Database, AuditTable) {
    let db = Database::open(path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new(handle).unwrap()).await.unwrap();
    (db.clone(), AuditTable::new(db, site))
}

#[cfg(test)]
fn file(dir: &tempfile::TempDir) -> PathBuf {
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    path
}

#[tokio::test]
async fn changes_come_back_newest_first_and_outlast_closing_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let (db, log) = table(&file(&dir), "main").await;
    let first = change(1, Action::Added, "/api/admin/v1/posts/");
    let second = change(2, Action::Changed, "/api/admin/v1/posts/en~hello/");
    let third = change(3, Action::Removed, "/api/admin/v1/posts/en~hello/");
    for kept in [&first, &second, &third] {
        log.record(kept).await.unwrap();
    }
    db.close().await;

    let (db, log) = table(&file(&dir), "main").await;
    let page = log.changes(None, 2).await.unwrap();
    let seen: Vec<&Change> = page.iter().map(|logged| &logged.change).collect();
    assert_eq!(seen, [&third, &second]);
    let rest = log.changes(Some(page[1].id), 2).await.unwrap();
    let seen: Vec<&Change> = rest.iter().map(|logged| &logged.change).collect();
    assert_eq!(seen, [&first]);
    db.close().await;
}

#[tokio::test]
async fn each_site_sees_only_its_own_changes() {
    let dir = tempfile::tempdir().unwrap();
    let (db, main) = table(&file(&dir), "main").await;
    let docs = AuditTable::new(
        db.clone(),
        db.site(&Slug::try_new("docs").unwrap()).await.unwrap(),
    );
    main.record(&change(1, Action::Added, "/api/admin/v1/posts/"))
        .await
        .unwrap();
    assert_eq!(docs.changes(None, 10).await.unwrap(), []);
    assert_eq!(main.changes(None, 10).await.unwrap().len(), 1);
    db.close().await;
}
