//! Where the Admin API reads and writes the site's other files: pages,
//! snippets and authors, each one markdown file per language.

use std::error::Error;
use std::fmt;

use chrono::{DateTime, Utc};
use khalid_core::Locale;

use crate::content_store::BoxFuture;

/// A file the Admin API keeps whole, in API shape: a page, a snippet or an
/// author. `New` is a checked new one, and `Edit` a checked change that
/// carries the `updated_at` of the copy it was made on.
pub trait Entry: fmt::Debug + Clone + Send + Sync + 'static {
    /// A new entry, checked at the edge.
    type New: Send + 'static;
    /// A change to an entry, checked at the edge.
    type Edit: Send + 'static;

    /// The identity: the language and the slug, such as `en.about`.
    fn id(&self) -> &str;

    /// When the entry last changed.
    fn updated_at(&self) -> DateTime<Utc>;

    /// The `updated_at` of the copy `edit` was made on.
    fn seen(edit: &Self::Edit) -> DateTime<Utc>;

    /// The language `new` asks for, or `None` for the site's default.
    fn language(new: &Self::New) -> Option<&Locale>;

    /// `new` as an entry in `language`, made at `at`.
    fn made(new: Self::New, language: Locale, at: DateTime<Utc>) -> Self;

    /// The same entry with `edit` applied at `at`.
    #[must_use]
    fn edited(self, edit: Self::Edit, at: DateTime<Utc>) -> Self;
}

/// The site's entries of one kind, for the Admin API. `khalid serve` keeps
/// them in the content files; `MemoryEntries`, from the `test-util`
/// feature, keeps them in memory for tests. A site has tens of each, so a
/// list comes whole.
pub trait EntryStore<T: Entry>: Send + Sync {
    /// Every entry, in every language, ordered by language and then slug.
    ///
    /// # Errors
    ///
    /// Returns [`EntryStoreError`] when the entries cannot be read.
    fn list(&self) -> BoxFuture<'_, Result<Vec<T>, EntryStoreError>>;

    /// The entry with identity `id`, or `None`.
    ///
    /// # Errors
    ///
    /// Returns [`EntryStoreError`] when the entries cannot be read.
    fn get<'a>(&'a self, id: &'a str) -> BoxFuture<'a, Result<Option<T>, EntryStoreError>>;

    /// Saves `new` as a new entry.
    ///
    /// # Errors
    ///
    /// Returns [`EntryError::Taken`] when an entry has its identity,
    /// [`EntryError::Unsupported`] when the site has no such language, and
    /// [`EntryError::Store`] when the entries cannot be read or written.
    fn create(&self, new: T::New) -> BoxFuture<'_, Result<T, EntryError>>;

    /// Applies `edit` to entry `id`, but only when it has not changed since
    /// the copy the edit was made on (BL-API-7). The check and the write
    /// happen together.
    ///
    /// # Errors
    ///
    /// Returns [`EntryError::NotFound`], [`EntryError::Stale`] naming the
    /// last change, and [`EntryError::Store`].
    fn update<'a>(&'a self, id: &'a str, edit: T::Edit) -> BoxFuture<'a, Result<T, EntryError>>;

    /// Removes entry `id` when it has not changed since `updated_at`.
    ///
    /// # Errors
    ///
    /// Returns [`EntryError::NotFound`], [`EntryError::Stale`] naming the
    /// last change, and [`EntryError::Store`].
    fn delete<'a>(
        &'a self,
        id: &'a str,
        updated_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), EntryError>>;
}

/// Why an entry was not saved or removed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EntryError {
    /// No entry has that identity.
    #[error("no entry has that identity")]
    NotFound,
    /// The entry changed after the copy the request was made on.
    #[error("the entry changed after the copy the request was made on")]
    Stale {
        /// When the entry last changed.
        updated_at: DateTime<Utc>,
    },
    /// Another entry has this identity.
    #[error("another entry has the identity {0}")]
    Taken(String),
    /// The store cannot keep the request, such as a language the site is
    /// not written in. Nothing was saved.
    #[error("the store cannot keep {field}: {why}")]
    Unsupported {
        /// The field, as the request names it, such as `language`.
        field: &'static str,
        /// Why, in one plain sentence for the person who sent it.
        why: &'static str,
    },
    /// The entries could not be read or written.
    #[error(transparent)]
    Store(#[from] EntryStoreError),
}

/// The entries could not be read or written.
#[derive(Debug, thiserror::Error)]
#[error("the site's files could not be read or written")]
pub struct EntryStoreError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl EntryStoreError {
    /// Wraps what the storage reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}
