//! The export of every table: one site's rows only, in a stable order,
//! and nothing secret (KH-GRO-6).

use std::path::PathBuf;

use khalid_core::Slug;
use khalid_db::{Cell, Database, DbError, SiteExport, SiteId, is_withheld_column};
use sqlx::ConnectOptions as _;
use sqlx::sqlite::SqliteConnectOptions;

/// A migrated database holding the fixture's row in every table, for two
/// sites, and site 1 in it.
#[cfg(test)]
async fn filled() -> (tempfile::TempDir, SiteExport) {
    let dir = tempfile::tempdir().unwrap();
    let mut path = PathBuf::from(dir.path());
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let options = SqliteConnectOptions::new()
        .filename(&path)
        .foreign_keys(true);
    let mut connection = options.connect().await.unwrap();
    sqlx::raw_sql(include_str!("fixtures/every_table.sql"))
        .execute(&mut connection)
        .await
        .unwrap();
    let main: SiteId = db.site(&"main".parse::<Slug>().unwrap()).await.unwrap();
    (dir, SiteExport::new(db, main))
}

/// Every row of `table` the export of site 1 gives.
#[cfg(test)]
async fn rows(export: &SiteExport, table: &str) -> Vec<Vec<Cell>> {
    let tables = export.tables().await.unwrap();
    let table = tables.iter().find(|t| t.name() == table).unwrap();
    let mut rows = Vec::new();
    export
        .each_row(table, |row| {
            rows.push(row.to_vec());
            Ok::<_, DbError>(())
        })
        .await
        .unwrap();
    rows
}

#[tokio::test]
async fn every_table_of_the_site_leaves_except_sign_in_material() {
    let (_dir, export) = filled().await;
    let names: Vec<String> = export
        .tables()
        .await
        .unwrap()
        .iter()
        .map(|table| table.name().to_owned())
        .collect();
    for table in [
        "sites",
        "posts",
        "redirects",
        "not_found_log",
        "subscribers",
    ] {
        assert!(names.contains(&table.to_owned()), "{table} is missing");
    }
    for table in ["sessions", "magic_links", "subscriber_tokens", "site_keys"] {
        assert!(!names.contains(&table.to_owned()), "{table} left");
    }
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "tables come in name order");
}

#[tokio::test]
async fn no_column_that_holds_a_secret_leaves() {
    let (_dir, export) = filled().await;
    for table in export.tables().await.unwrap() {
        for column in table.columns() {
            assert!(!is_withheld_column(column), "{}.{column}", table.name());
        }
        if table.name() == "webhooks" {
            assert!(!table.columns().contains(&"secret".to_owned()));
            assert!(table.columns().contains(&"url".to_owned()));
        }
    }
    for column in ["secret", "key_hash", "token_hash", "password_hash"] {
        assert!(is_withheld_column(column), "{column}");
    }
    assert!(!is_withheld_column("content_hash"));
}

#[tokio::test]
async fn rows_are_the_sites_own_with_their_values() {
    let (_dir, export) = filled().await;
    let sites = rows(&export, "sites").await;
    assert_eq!(
        sites,
        [vec![Cell::Integer(1), Cell::Text("main".to_owned())]]
    );
    let posts = rows(&export, "posts").await;
    assert_eq!(posts.len(), 1, "site 2's post stayed out");
    let row = &posts[0];
    assert!(row.contains(&Cell::Text("old-post".to_owned())));
    assert!(row.contains(&Cell::Null), "no description is no value");
    assert!(row.contains(&Cell::Bytes(vec![0; 32])), "the content hash");
    assert_eq!(Cell::Bytes(vec![0, 255]).text(), "00ff");
    assert_eq!(Cell::Null.text(), "");
}
