//! Why a database step failed.

use std::path::PathBuf;

/// Why the database could not be opened, migrated or queried.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DbError {
    /// The file could not be opened as a SQLite database with the settings
    /// Khalid needs.
    #[error("could not open the database {}", path.display())]
    Open {
        /// The database file.
        path: PathBuf,
        /// What SQLite reported.
        #[source]
        source: sqlx::Error,
    },
    /// A migration could not be applied, or the file holds a migration this
    /// version of Khalid does not know.
    #[error("could not bring the database schema up to date")]
    Migrate {
        /// What the migrator reported.
        #[source]
        source: sqlx::migrate::MigrateError,
    },
}
