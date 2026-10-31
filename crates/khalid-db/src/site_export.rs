//! Every table of one site read out row by row, for `khalid db export`
//! and the spreadsheets and Looker Studio reports it feeds (KH-GRO-6).
//!
//! Nothing secret leaves: a table that only holds sign-in material is left
//! out whole, and a column that holds a secret or its hash is left out of
//! its table. Rows of other sites in the same file are never read.

use std::fmt::Write as _;

use futures_util::TryStreamExt as _;
use sqlx::sqlite::SqliteRow;
use sqlx::{AssertSqlSafe, Row as _, TypeInfo as _, ValueRef as _};

use crate::{Database, DbError, SiteId};

/// Tables of sign-in material only: sessions, sign-in and confirmation
/// links, passkeys and the site's own signing keys.
const WITHHELD_TABLES: [&str; 5] = [
    "magic_links",
    "passkeys",
    "sessions",
    "site_keys",
    "subscriber_tokens",
];

/// Whether a column named `name` holds a secret or a hash of one, and so
/// never leaves: `secret`, `password_hash`, `key_hash`, `token_hash`, and
/// any later column named the same way. A content hash is no secret.
pub fn is_withheld_column(name: &str) -> bool {
    let secretive = ["secret", "token", "password"];
    secretive.iter().any(|word| name.contains(word))
        || (name.ends_with("_hash") && name != "content_hash")
}

/// One table as it is exported: its name and the columns that leave, in
/// the schema's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportTable {
    name: String,
    columns: Vec<String>,
    /// The column that names the site: `id` in `sites`, `site_id` elsewhere.
    site_column: &'static str,
}

impl ExportTable {
    /// The table's name, such as `not_found_log`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The columns that leave, in the schema's order.
    pub fn columns(&self) -> &[String] {
        &self.columns
    }
}

/// One value of one row. A blob, such as a content hash, is read out as
/// lowercase hex, so CSV and JSON can both carry it.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// No value.
    Null,
    /// A whole number.
    Integer(i64),
    /// A number with a fraction.
    Real(f64),
    /// Text.
    Text(String),
    /// Bytes.
    Bytes(Vec<u8>),
}

impl Cell {
    /// The value as text for a CSV field: empty for no value, hex for
    /// bytes.
    pub fn text(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Integer(number) => number.to_string(),
            Self::Real(number) => number.to_string(),
            Self::Text(text) => text.clone(),
            Self::Bytes(bytes) => hex(bytes),
        }
    }
}

/// The tables of one site in a database, ready to read out. Cloning shares
/// the connections.
#[derive(Debug, Clone)]
pub struct SiteExport {
    db: Database,
    site: SiteId,
}

impl SiteExport {
    /// The export of `site` in `db`.
    pub fn new(db: Database, site: SiteId) -> Self {
        Self { db, site }
    }

    /// Every table that leaves, by name, with the columns that leave. A
    /// table with no column naming its site is not one site's, so it stays.
    ///
    /// # Errors
    ///
    /// [`DbError::Query`] when the schema cannot be read.
    pub async fn tables(&self) -> Result<Vec<ExportTable>, DbError> {
        let names: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_schema WHERE type = 'table' \
             AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '\\_sqlx%' ESCAPE '\\' \
             ORDER BY name",
        )
        .fetch_all(&self.db.readers)
        .await?;
        let mut tables = Vec::new();
        for name in names {
            if WITHHELD_TABLES.contains(&name.as_str()) {
                continue;
            }
            let all: Vec<String> =
                sqlx::query_scalar("SELECT name FROM pragma_table_info(?1) ORDER BY cid")
                    .bind(&name)
                    .fetch_all(&self.db.readers)
                    .await?;
            let site_column = if name == "sites" { "id" } else { "site_id" };
            if !all.iter().any(|column| column == site_column) {
                continue;
            }
            let columns = all.into_iter().filter(|c| !is_withheld_column(c)).collect();
            tables.push(ExportTable {
                name,
                columns,
                site_column,
            });
        }
        Ok(tables)
    }

    /// Hands each of the site's rows in `table` to `each`, in order of
    /// their columns, one at a time, so a large table is never held whole.
    ///
    /// # Errors
    ///
    /// [`DbError::Query`] when a row cannot be read, and whatever `each`
    /// returns, which stops the reading.
    pub async fn each_row<E: From<DbError>>(
        &self,
        table: &ExportTable,
        mut each: impl FnMut(&[Cell]) -> Result<(), E> + Send,
    ) -> Result<(), E> {
        let columns: Vec<String> = table.columns.iter().map(|c| quoted(c)).collect();
        let mut sql = format!(
            "SELECT {} FROM {} WHERE {} = ?1 ORDER BY ",
            columns.join(", "),
            quoted(&table.name),
            table.site_column
        );
        let order: Vec<String> = (1..=columns.len()).map(|n| n.to_string()).collect();
        sql.push_str(&order.join(", "));
        // The names come from the schema and are quoted; the site is bound.
        let mut rows = sqlx::query(AssertSqlSafe(sql))
            .bind(self.site.as_row())
            .fetch(&self.db.readers);
        while let Some(row) = rows.try_next().await.map_err(DbError::from)? {
            let cells = (0..columns.len())
                .map(|index| cell(&row, index))
                .collect::<Result<Vec<_>, DbError>>()?;
            each(&cells)?;
        }
        Ok(())
    }
}

/// The value in column `index` of `row`, as SQLite stored it.
fn cell(row: &SqliteRow, index: usize) -> Result<Cell, DbError> {
    let raw = row.try_get_raw(index)?;
    if raw.is_null() {
        return Ok(Cell::Null);
    }
    let kind = raw.type_info().name().to_owned();
    Ok(match kind.as_str() {
        "INTEGER" => Cell::Integer(row.try_get(index)?),
        "REAL" => Cell::Real(row.try_get(index)?),
        "BLOB" => Cell::Bytes(row.try_get(index)?),
        _ => Cell::Text(row.try_get(index)?),
    })
}

/// `name` as an SQL identifier in double quotes.
fn quoted(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// `bytes` as lowercase hex.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}
