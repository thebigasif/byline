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
    /// A query failed: the file is locked for longer than the busy timeout,
    /// the disk is full, or a row breaks a rule of the schema.
    #[error("a database query failed")]
    Query(#[from] sqlx::Error),
    /// A webhook delivery was updated that the site's log does not hold.
    #[error("the delivery log has no delivery {id}")]
    NoSuchDelivery {
        /// The delivery's id.
        id: i64,
    },
    /// Something else changed a stored value; a rebuild writes it again.
    #[error("the database holds a {column} Khalid cannot read: {value:?}")]
    Unreadable {
        /// The column the value came from.
        column: &'static str,
        /// The value, or a description of it when it is not text.
        value: String,
    },
}
