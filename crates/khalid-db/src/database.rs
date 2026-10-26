//! The open database: one writer connection and a pool of readers.

use std::path::Path;
use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

use crate::DbError;

/// The migrations in `crates/khalid-db/migrations/`, built into the binary.
static MIGRATOR: Migrator = sqlx::migrate!();

/// How long a connection waits for another one's lock before it gives up.
/// Long enough for any single write Khalid makes, short enough that a stuck
/// writer shows up as an error instead of a hang.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// How many read connections may be open at once. Reads never wait for each
/// other in WAL mode, so this caps memory, not throughput.
const READERS: u32 = 8;

/// A site's database file, opened with the settings RULES.md BL-DB-1 asks for:
/// WAL mode, foreign keys on, a busy timeout, one writer and a read pool.
///
/// Every write goes through the one writer connection, so writes never wait
/// on each other's locks. Reads go through the pool and never block a write.
#[derive(Debug, Clone)]
pub struct Database {
    pub(crate) writer: SqlitePool,
    pub(crate) readers: SqlitePool,
}

impl Database {
    /// Opens the database file at `path`, creating it when it does not exist.
    /// The folder it sits in must exist. Call [`Database::migrate`] before the
    /// first query.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Open`] when the folder is missing, the file is not a
    /// SQLite database, or a setting cannot be applied.
    pub async fn open(path: &Path) -> Result<Self, DbError> {
        let open_error = |source| DbError::Open {
            path: path.to_path_buf(),
            source,
        };
        let options = SqliteConnectOptions::new()
            .filename(path)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .foreign_keys(true)
            .busy_timeout(BUSY_TIMEOUT);
        // The writer opens first: it creates the file and switches it to WAL,
        // which read-only connections cannot do.
        let writer = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options.clone().create_if_missing(true))
            .await
            .map_err(open_error)?;
        let readers = SqlitePoolOptions::new()
            .max_connections(READERS)
            .connect_with(options.read_only(true))
            .await
            .map_err(open_error)?;
        Ok(Self { writer, readers })
    }

    /// Applies every migration the file does not have yet, in order. A file
    /// that is already up to date is left as it is.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Migrate`] when a migration fails, or when the file
    /// was migrated by a newer Khalid or an edited migration (BL-DB-2).
    pub async fn migrate(&self) -> Result<(), DbError> {
        MIGRATOR
            .run(&self.writer)
            .await
            .map_err(|source| DbError::Migrate { source })
    }

    /// Waits for running queries to finish, then closes every connection, so
    /// the write-ahead log is folded back into the file (BL-RUN-10). Clones
    /// share their connections, so this closes them for every clone.
    pub async fn close(self) {
        self.readers.close().await;
        self.writer.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn opened() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().unwrap();
        let mut path = dir.path().to_path_buf();
        path.push("site.db");
        let db = Database::open(&path).await.unwrap();
        db.migrate().await.unwrap();
        (dir, db)
    }

    async fn pragma(pool: &SqlitePool, sql: &'static str) -> String {
        let row: (String,) = sqlx::query_as(sql).fetch_one(pool).await.unwrap();
        row.0
    }

    #[tokio::test]
    async fn every_connection_uses_wal_foreign_keys_and_a_busy_timeout() {
        let (_dir, db) = opened().await;
        // synchronous = 1 is NORMAL: safe in WAL mode, and a commit does not
        // wait on a sync.
        let expected = [
            ("SELECT * FROM pragma_journal_mode", "wal"),
            (
                "SELECT CAST(foreign_keys AS TEXT) FROM pragma_foreign_keys",
                "1",
            ),
            (
                "SELECT CAST(timeout AS TEXT) FROM pragma_busy_timeout",
                "5000",
            ),
            (
                "SELECT CAST(synchronous AS TEXT) FROM pragma_synchronous",
                "1",
            ),
        ];
        for pool in [&db.writer, &db.readers] {
            for (sql, value) in expected {
                assert_eq!(pragma(pool, sql).await, value, "{sql}");
            }
        }
        db.close().await;
    }

    #[tokio::test]
    async fn there_is_one_writer_and_readers_cannot_write() {
        let (_dir, db) = opened().await;
        assert_eq!(db.writer.options().get_max_connections(), 1);
        assert_eq!(db.readers.options().get_max_connections(), READERS);
        let refused = sqlx::query("INSERT INTO sites (handle) VALUES ('main')")
            .execute(&db.readers)
            .await;
        assert!(refused.is_err(), "a reader wrote to the database");
        sqlx::query("INSERT INTO sites (handle) VALUES ('main')")
            .execute(&db.writer)
            .await
            .unwrap();
        db.close().await;
    }

    #[tokio::test]
    async fn a_file_from_a_newer_khalid_is_refused() {
        let (_dir, db) = opened().await;
        sqlx::query(
            "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) \
             VALUES (9999, 'from the future', TRUE, X'00', 0)",
        )
        .execute(&db.writer)
        .await
        .unwrap();
        let error = db.migrate().await.unwrap_err();
        assert!(matches!(error, DbError::Migrate { .. }), "{error:?}");
        db.close().await;
    }
}
