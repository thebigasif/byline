//! Opening the database and running its migrations (RULES.md BL-DB-1, BL-DB-2).

use std::path::{Path, PathBuf};

use khalid_db::{Database, DbError};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions, Connection};

/// `name` inside `dir`.
#[cfg(test)]
fn within(dir: &Path, name: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(name);
    path
}

/// The versions of the migrations applied to the file, read with a plain
/// connection so the test does not depend on the code it checks.
#[cfg(test)]
async fn applied(path: &Path) -> Vec<i64> {
    let mut connection = SqliteConnectOptions::new()
        .filename(path)
        .connect()
        .await
        .unwrap();
    let versions = sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
        .fetch_all(&mut connection)
        .await
        .unwrap();
    connection.close().await.unwrap();
    versions
}

#[tokio::test]
async fn migrate_applies_every_migration_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = within(dir.path(), "site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    db.migrate().await.unwrap();
    db.close().await;
    let again = Database::open(&path).await.unwrap();
    again.migrate().await.unwrap();
    again.close().await;

    let migrations = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations"))
        .unwrap()
        .count();
    let applied = applied(&path).await;
    assert_eq!(applied.len(), migrations, "{applied:?}");
    assert!(applied.is_sorted());
}

#[tokio::test]
async fn a_missing_folder_is_an_open_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = within(dir.path(), "no-such-folder/site.db");
    let error = Database::open(&path).await.unwrap_err();
    assert!(
        matches!(&error, DbError::Open { path: p, .. } if *p == path),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_file_that_is_not_a_database_is_an_open_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = within(dir.path(), "site.db");
    std::fs::write(&path, "this is a text file, not a database").unwrap();
    let error = Database::open(&path).await.unwrap_err();
    assert!(matches!(error, DbError::Open { .. }), "{error:?}");
}
