//! Entries held in memory, for tests.

use std::sync::{Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, Utc};
use khalid_core::{Clock, Locale, SystemClock};

use super::{Entry, EntryError, EntryStore, EntryStoreError};
use crate::content_store::BoxFuture;

/// Entries of one kind held in memory, for tests. A new entry with no
/// language is in English, as on a site with the default settings.
#[derive(Debug)]
pub struct MemoryEntries<T> {
    entries: Mutex<Vec<T>>,
    clock: Box<dyn Clock>,
}

impl<T: Entry> MemoryEntries<T> {
    /// A store holding `entries`, stamping changes with the system clock.
    pub fn new(entries: Vec<T>) -> Self {
        Self {
            entries: Mutex::new(entries),
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

    fn entries(&self) -> MutexGuard<'_, Vec<T>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn add(&self, new: T::New) -> Result<T, EntryError> {
        let language = T::language(&new).cloned();
        let english = Locale::try_new("en").map_err(EntryStoreError::new)?;
        let entry = T::made(new, language.unwrap_or(english), self.clock.now());
        let mut entries = self.entries();
        if entries.iter().any(|kept| kept.id() == entry.id()) {
            return Err(EntryError::Taken(entry.id().to_owned()));
        }
        entries.push(entry.clone());
        Ok(entry)
    }

    /// The position of entry `id`, when it has not changed since `seen`.
    fn current(entries: &[T], id: &str, seen: DateTime<Utc>) -> Result<usize, EntryError> {
        let at = entries
            .iter()
            .position(|entry| entry.id() == id)
            .ok_or(EntryError::NotFound)?;
        let updated_at = entries.get(at).map_or(seen, Entry::updated_at);
        if updated_at == seen {
            Ok(at)
        } else {
            Err(EntryError::Stale { updated_at })
        }
    }

    fn change(&self, id: &str, edit: T::Edit) -> Result<T, EntryError> {
        let mut entries = self.entries();
        let at = Self::current(&entries, id, T::seen(&edit))?;
        let entry = entries.get_mut(at).ok_or(EntryError::NotFound)?;
        *entry = entry.clone().edited(edit, self.clock.now());
        Ok(entry.clone())
    }

    fn remove(&self, id: &str, seen: DateTime<Utc>) -> Result<(), EntryError> {
        let mut entries = self.entries();
        let at = Self::current(&entries, id, seen)?;
        entries.remove(at);
        Ok(())
    }
}

/// Memory answers at once, so each future is ready when returned.
impl<T: Entry> EntryStore<T> for MemoryEntries<T> {
    fn list(&self) -> BoxFuture<'_, Result<Vec<T>, EntryStoreError>> {
        Box::pin(std::future::ready(Ok(self.entries().clone())))
    }

    fn get<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Option<T>, EntryStoreError>> {
        let found = self
            .entries()
            .iter()
            .find(|entry| entry.id() == id)
            .cloned();
        Box::pin(std::future::ready(Ok(found)))
    }

    fn create(&self, new: T::New) -> BoxFuture<'_, Result<T, EntryError>> {
        Box::pin(std::future::ready(self.add(new)))
    }

    fn update<'a>(&'a self, id: &'a str, edit: T::Edit) -> BoxFuture<'a, Result<T, EntryError>> {
        Box::pin(std::future::ready(self.change(id, edit)))
    }

    fn delete<'a>(
        &'a self,
        id: &'a str,
        updated_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), EntryError>> {
        Box::pin(std::future::ready(self.remove(id, updated_at)))
    }
}
