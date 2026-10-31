//! Settings held in memory, for tests.

use std::sync::{Mutex, MutexGuard, PoisonError};

use khalid_core::{Clock, SystemClock};

use super::{SettingsError, SettingsStore};
use crate::content_store::BoxFuture;
use crate::entry_store::EntryStoreError;
use crate::resources::{AdminSettings, SettingsChange};

/// Settings held in memory, for tests. A change applies at once, so a
/// restart is never needed; it refuses a default language missing from
/// the languages, as the checks at start do.
#[derive(Debug)]
pub struct MemorySettings {
    settings: Mutex<AdminSettings>,
    clock: Box<dyn Clock>,
}

impl MemorySettings {
    /// A store holding `settings`, stamping changes with the system clock.
    pub fn new(settings: AdminSettings) -> Self {
        Self {
            settings: Mutex::new(settings),
            clock: Box::new(SystemClock),
        }
    }

    /// The same store, stamping changes with `clock`.
    #[must_use]
    pub fn with_clock(self, clock: impl Clock + 'static) -> Self {
        Self {
            clock: Box::new(clock),
            ..self
        }
    }

    fn settings(&self) -> MutexGuard<'_, AdminSettings> {
        self.settings.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn apply(&self, change: SettingsChange) -> Result<AdminSettings, SettingsError> {
        let mut settings = self.settings();
        if settings.updated_at != change.updated_at {
            let updated_at = settings.updated_at;
            return Err(SettingsError::Stale { updated_at });
        }
        let mut next = settings.clone();
        next.name = change.name.unwrap_or(next.name);
        next.address = change.address.unwrap_or(next.address);
        next.mount = change.mount.unwrap_or(next.mount);
        next.default_language = change.default_language.unwrap_or(next.default_language);
        next.languages = change.languages.unwrap_or(next.languages);
        if !next.languages.contains(&next.default_language) {
            return Err(SettingsError::Refused {
                field: "default_language".to_owned(),
                problem: "the default language must be one of the languages".to_owned(),
            });
        }
        next.updated_at = self.clock.now();
        *settings = next.clone();
        Ok(next)
    }
}

/// Memory answers at once, so each future is ready when returned.
impl SettingsStore for MemorySettings {
    fn read(&self) -> BoxFuture<'_, Result<AdminSettings, EntryStoreError>> {
        Box::pin(std::future::ready(Ok(self.settings().clone())))
    }

    fn change(
        &self,
        change: SettingsChange,
    ) -> BoxFuture<'_, Result<AdminSettings, SettingsError>> {
        Box::pin(std::future::ready(self.apply(change)))
    }
}
