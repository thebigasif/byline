//! Where the API reads published content from.

use std::error::Error;
use std::future::Future;
use std::pin::Pin;

use khalid_core::Slug;

use crate::query::ListQuery;
use crate::resources::{Collection, Post};

mod memory;
pub use memory::MemoryContentStore;

/// A future the store returns, boxed so routes can hold any store as a
/// `dyn ContentStore`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Published content, read by the Content API. `MemoryContentStore` keeps
/// entries in memory: `khalid serve` fills one from the content index in
/// `khalid-db` after each change, and tests fill one by hand.
///
/// A store applies the query's filter, order and page itself, so a
/// database can do it in SQL. Fields named in the query have already been
/// checked against [`Post::FILTER_FIELDS`].
pub trait ContentStore: Send + Sync {
    /// One page of the published entries of `collection` matching `query`,
    /// newest first unless `query` orders them, with the number that match
    /// in all.
    ///
    /// # Errors
    ///
    /// Returns [`ContentStoreError`] when the content cannot be read.
    fn list<'a>(
        &'a self,
        collection: Collection,
        query: &'a ListQuery,
    ) -> BoxFuture<'a, Result<Listed<Post>, ContentStoreError>>;

    /// One published entry of `collection`, or `None` when nothing in it
    /// has that key.
    ///
    /// # Errors
    ///
    /// Returns [`ContentStoreError`] when the content cannot be read.
    fn find<'a>(
        &'a self,
        collection: Collection,
        key: &'a PostKey,
    ) -> BoxFuture<'a, Result<Option<Post>, ContentStoreError>>;
}

/// How one entry is asked for: `/posts/{id}/` or `/posts/slug/{slug}/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostKey {
    /// By identity.
    Id(String),
    /// By slug.
    Slug(Slug),
}

/// One page of a list, and how many items the whole list has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed<T> {
    items: Vec<T>,
    total: u64,
}

impl<T> Listed<T> {
    /// `items` from a list of `total`.
    pub fn new(items: Vec<T>, total: u64) -> Self {
        Self { items, total }
    }

    /// This page's items.
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// How many items the whole list has.
    pub fn total(&self) -> u64 {
        self.total
    }

    /// The items, owned.
    pub fn into_items(self) -> Vec<T> {
        self.items
    }
}

/// The content could not be read.
#[derive(Debug, thiserror::Error)]
#[error("the published content could not be read")]
pub struct ContentStoreError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl ContentStoreError {
    /// Wraps what the storage reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}
