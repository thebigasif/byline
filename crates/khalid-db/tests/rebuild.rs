//! The rebuild: a site's derived tables are emptied and filled again from the
//! content index, its visitor tables are kept, and other sites are untouched
//! (ADR 0001, ADR 0010).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use khalid_core::Slug;
use khalid_db::{ContentIndex, Database, SiteId};
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};
use sqlx::{AssertSqlSafe, ConnectOptions};

/// A migrated database in a fresh folder.
#[cfg(test)]
async fn migrated() -> (tempfile::TempDir, PathBuf, Database) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    (dir, path, db)
}

/// A plain connection, so the checks do not depend on the code they check.
#[cfg(test)]
async fn connect(path: &Path) -> SqliteConnection {
    SqliteConnectOptions::new()
        .filename(path)
        .foreign_keys(true)
        .connect()
        .await
        .unwrap()
}

/// The tables the crate README lists as `kind`.
#[cfg(test)]
fn readme_tables(kind: &str) -> Vec<String> {
    include_str!("../README.md")
        .lines()
        .filter_map(|line| line.strip_prefix("| `"))
        .filter_map(|rest| rest.split_once("` | "))
        .filter(|(_, rest)| rest.starts_with(&format!("{kind} |")))
        .map(|(name, _)| name.to_owned())
        .collect()
}

/// How many rows each listed table holds for `site`.
#[cfg(test)]
async fn counts(
    connection: &mut SqliteConnection,
    tables: &[String],
    site: i64,
) -> BTreeMap<String, i64> {
    let mut counts = BTreeMap::new();
    for table in tables {
        let column = if table == "sites" { "id" } else { "site_id" };
        // Table names come from the README, never from input.
        let sql = format!("SELECT count(*) FROM {table} WHERE {column} = ?");
        let count: i64 = sqlx::query_scalar(AssertSqlSafe(sql))
            .bind(site)
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        counts.insert(table.clone(), count);
    }
    counts
}

#[tokio::test]
async fn a_rebuild_empties_derived_rows_and_keeps_visitor_rows() {
    let (_dir, path, db) = migrated().await;
    let mut connection = connect(&path).await;
    sqlx::raw_sql(include_str!("fixtures/every_table.sql"))
        .execute(&mut connection)
        .await
        .unwrap();
    let derived = readme_tables("derived");
    let visitor = readme_tables("visitor");
    let derived_before = counts(&mut connection, &derived, 1).await;
    let visitor_before = counts(&mut connection, &visitor, 1).await;
    let sister_before = counts(&mut connection, &derived, 2).await;
    // A table the fixture leaves empty would prove nothing below.
    for (table, count) in derived_before.iter().chain(&visitor_before) {
        assert!(*count > 0, "the fixture has no row in {table}");
    }

    // The fixture made `main` as site 1. Were a second row made instead, site
    // 1's derived rows would survive and the check below would fail.
    let site: SiteId = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    db.rebuild(site, &ContentIndex::default()).await.unwrap();

    for (table, count) in counts(&mut connection, &derived, 1).await {
        assert_eq!(count, 0, "{table} still has rows after the rebuild");
    }
    assert_eq!(counts(&mut connection, &visitor, 1).await, visitor_before);
    assert_eq!(counts(&mut connection, &derived, 2).await, sister_before);
    db.close().await;
}

#[tokio::test]
async fn asking_for_a_site_twice_gives_the_same_id() {
    let (_dir, _path, db) = migrated().await;
    let main = Slug::try_new("main").unwrap();
    let first = db.site(&main).await.unwrap();
    let again = db.site(&main).await.unwrap();
    let other = db.site(&Slug::try_new("sister").unwrap()).await.unwrap();
    assert_eq!(first, again);
    assert_ne!(first, other);
    db.close().await;
}
