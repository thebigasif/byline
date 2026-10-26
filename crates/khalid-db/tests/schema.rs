//! The schema: every table is listed in the README as derived or visitor, and
//! every table belongs to a site (ADR 0009).

use std::collections::BTreeSet;
use std::path::PathBuf;

use khalid_db::Database;
use sqlx::ConnectOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};

/// A migrated database in a fresh folder, and a plain connection to it with
/// foreign keys on, so the checks do not depend on the code they check.
#[cfg(test)]
async fn migrated() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    db.close().await;
    let connection = SqliteConnectOptions::new()
        .filename(&path)
        .foreign_keys(true)
        .connect()
        .await
        .unwrap();
    (dir, connection)
}

/// Every table in the schema, apart from SQLite's and the migrator's own.
#[cfg(test)]
async fn tables(connection: &mut SqliteConnection) -> BTreeSet<String> {
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_schema WHERE type = 'table' \
         AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx_%'",
    )
    .fetch_all(connection)
    .await
    .unwrap();
    names.into_iter().collect()
}

/// The rows of the table in `crates/khalid-db/README.md`: each table's name
/// and whether it is derived or visitor data.
#[cfg(test)]
fn readme_tables() -> Vec<(String, String)> {
    include_str!("../README.md")
        .lines()
        .filter_map(|line| line.strip_prefix("| `"))
        .map(|rest| {
            let (name, rest) = rest.split_once("` | ").unwrap();
            let (kind, _) = rest.split_once(" | ").unwrap();
            (name.to_owned(), kind.to_owned())
        })
        .collect()
}

#[tokio::test]
async fn the_readme_lists_every_table_once_as_derived_or_visitor() {
    let (_dir, mut connection) = migrated().await;
    let listed = readme_tables();
    let names: BTreeSet<String> = listed.iter().map(|(name, _)| name.clone()).collect();
    assert_eq!(names.len(), listed.len(), "a table is listed twice");
    assert_eq!(tables(&mut connection).await, names);
    for (name, kind) in &listed {
        assert!(
            kind == "derived" || kind == "visitor",
            "{name} is {kind:?}, not derived or visitor"
        );
    }
}

#[tokio::test]
async fn every_table_has_a_site_id_that_points_at_sites() {
    let (_dir, mut connection) = migrated().await;
    for table in tables(&mut connection).await {
        if table == "sites" {
            continue;
        }
        let points: Option<String> = sqlx::query_scalar(
            "SELECT \"table\" FROM pragma_foreign_key_list(?) \
             WHERE \"from\" = 'site_id' AND \"table\" = 'sites'",
        )
        .bind(&table)
        .fetch_optional(&mut connection)
        .await
        .unwrap();
        assert!(points.is_some(), "{table} has no site_id pointing at sites");
    }
}

#[tokio::test]
async fn a_row_for_a_site_that_does_not_exist_is_refused() {
    let (_dir, mut connection) = migrated().await;
    let orphan = sqlx::query(
        "INSERT INTO tags (site_id, locale, slug, name) VALUES (99, 'en', 'seo', 'SEO')",
    )
    .execute(&mut connection)
    .await;
    assert!(orphan.is_err(), "a tag was written for a missing site");
}

#[tokio::test]
async fn a_value_of_the_wrong_kind_is_refused() {
    let (_dir, mut connection) = migrated().await;
    sqlx::query("INSERT INTO sites (id, handle) VALUES (1, 'main')")
        .execute(&mut connection)
        .await
        .unwrap();
    let short_hash = sqlx::query(
        "INSERT INTO pages (site_id, locale, slug, path, content_hash) \
         VALUES (1, 'en', 'about', 'content/en/pages/about.md', X'00')",
    )
    .execute(&mut connection)
    .await;
    assert!(
        short_hash.is_err(),
        "a page was stored with a one-byte hash"
    );
    let bad_date = sqlx::query(
        "INSERT INTO posts (site_id, locale, collection, slug, path, date, content_hash) \
         VALUES (1, 'en', 'posts', 'hello', 'content/en/posts/hello.md', '2026-02-30', zeroblob(32))",
    )
    .execute(&mut connection)
    .await;
    assert!(
        bad_date.is_err(),
        "a post was stored with a day that does not exist"
    );
}
