//! Where the Admin API reads and changes the site's core settings.

use chrono::{DateTime, Utc};

use crate::content_store::BoxFuture;
use crate::entry_store::EntryStoreError;
use crate::resources::{AdminSettings, SettingsChange};

#[cfg(any(test, feature = "test-util"))]
mod memory;
#[cfg(any(test, feature = "test-util"))]
pub use memory::MemorySettings;

/// The site's core settings, for the Admin API. `khalid serve` keeps them
/// in `site.toml`; `MemorySettings`, from the `test-util` feature, keeps
/// them in memory for tests.
pub trait SettingsStore: Send + Sync {
    /// The settings as `site.toml` holds them now.
    ///
    /// # Errors
    ///
    /// Returns [`EntryStoreError`] when the settings cannot be read.
    fn read(&self) -> BoxFuture<'_, Result<AdminSettings, EntryStoreError>>;

    /// Applies `change`, but only when the settings have not changed since
    /// the copy it was made on (BL-API-7), and only when every value would
    /// pass the checks `khalid serve` makes at start.
    ///
    /// # Errors
    ///
    /// Returns [`SettingsError::Stale`], [`SettingsError::Refused`] naming
    /// the field, and [`SettingsError::Store`].
    fn change(&self, change: SettingsChange)
    -> BoxFuture<'_, Result<AdminSettings, SettingsError>>;
}

/// Why a change to the settings was not saved.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SettingsError {
    /// The settings changed after the copy the change was made on.
    #[error("the settings changed after the copy the change was made on")]
    Stale {
        /// When they last changed.
        updated_at: DateTime<Utc>,
    },
    /// A value would not pass the checks at start. Nothing was saved.
    #[error("{field}: {problem}")]
    Refused {
        /// The field, as `site.toml` names it, such as `languages`.
        field: String,
        /// What is wrong with it, in one plain sentence.
        problem: String,
    },
    /// The settings could not be read or written.
    #[error(transparent)]
    Store(#[from] EntryStoreError),
}
