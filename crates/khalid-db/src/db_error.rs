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
    /// A copy of the database could not be written: the file is already
    /// there, its folder is missing, or the disk is full.
    #[error("could not copy the database to {}", path.display())]
    Copy {
        /// Where the copy was to go.
        path: PathBuf,
        /// What SQLite reported.
        #[source]
        source: sqlx::Error,
    },
    /// SQLite takes a file name as text, and this path is not valid text.
    #[error("the path {} is not valid text, so SQLite cannot use it", path.display())]
    PathNotText {
        /// The path.
        path: PathBuf,
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
    /// The system gave no randomness to make a webhook's secret from.
    #[error("the system has no randomness to make a secret from")]
    NoRandomness {
        /// What the system reported.
        #[source]
        source: getrandom::Error,
    },
    /// An unsubscribe link was added for a subscriber the site does not
    /// have.
    #[error("the site has no subscriber {id}")]
    NoSuchSubscriber {
        /// The subscriber's row.
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
