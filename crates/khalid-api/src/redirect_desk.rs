//! What the redirect routes read and write: the stored redirects, the 404
//! log, the addresses of the site's pages, and the clock (KH-SEO-8).

use std::error::Error;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use khalid_core::{Clock, SystemClock};
use khalid_seo::{RedirectEntry, RedirectTable, SeoError, UrlPath};

use crate::content_store::BoxFuture;
use crate::not_found_log::{Miss, NotFoundLog, NotFoundLogError};
use crate::redirect_store::{Origin, RedirectStore, RedirectStoreError, StoredRedirect};

/// The addresses of the pages a missing address may be sent to.
pub trait PageAddresses: Send + Sync {
    /// Every page's address, from the site root, in a stable order.
    ///
    /// # Errors
    ///
    /// [`PageAddressesError`] when the pages cannot be listed.
    fn addresses(&self) -> BoxFuture<'_, Result<Vec<UrlPath>, PageAddressesError>>;
}

/// The pages could not be listed.
#[derive(Debug, thiserror::Error)]
#[error("the site's pages could not be listed")]
pub struct PageAddressesError {
    #[source]
    source: Box<dyn Error + Send + Sync>,
}

impl PageAddressesError {
    /// Wraps what the site reported.
    pub fn new(source: impl Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

/// A fixed list of page addresses.
#[derive(Debug, Clone, Default)]
pub struct StaticPages(Vec<UrlPath>);

impl StaticPages {
    /// The pages at `addresses`.
    ///
    /// # Errors
    ///
    /// [`SeoError::Path`] when an address is not a clean path.
    pub fn new(addresses: &[&str]) -> Result<Self, SeoError> {
        let paths = addresses.iter().map(|a| UrlPath::try_new(a));
        Ok(Self(paths.collect::<Result<_, _>>()?))
    }
}

impl PageAddresses for StaticPages {
    fn addresses(&self) -> BoxFuture<'_, Result<Vec<UrlPath>, PageAddressesError>> {
        Box::pin(std::future::ready(Ok(self.0.clone())))
    }
}

/// Everything the redirect routes need. Cloning shares it.
#[derive(Clone)]
pub struct RedirectDesk {
    pub(crate) store: Arc<dyn RedirectStore>,
    pub(crate) log: Arc<dyn NotFoundLog>,
    pub(crate) pages: Arc<dyn PageAddresses>,
    pub(crate) clock: Arc<dyn Clock>,
}

/// Why redirects were not stored. Nothing is stored when any is refused.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreRefusal {
    /// With the stored redirects they make no table: a loop, or two rules
    /// for one address.
    #[error(transparent)]
    Table(SeoError),
    /// An address already has a stored redirect.
    #[error("the address {from:?} already has a redirect")]
    Taken {
        /// The address, or the pattern as written.
        from: String,
    },
    /// The store could not be read or written.
    #[error(transparent)]
    Storage(RedirectStoreError),
}

impl From<RedirectStoreError> for StoreRefusal {
    fn from(error: RedirectStoreError) -> Self {
        match error {
            RedirectStoreError::Taken { from } => Self::Taken { from },
            error => Self::Storage(error),
        }
    }
}

impl RedirectDesk {
    /// The table every stored redirect makes, as the server answers it.
    ///
    /// # Errors
    ///
    /// [`StoreRefusal::Storage`] when the store cannot be read, and
    /// [`StoreRefusal::Table`] when the stored rows make no table.
    pub async fn table(&self) -> Result<RedirectTable, StoreRefusal> {
        let stored = self.store.list().await?;
        let entries = stored.into_iter().map(|s| s.entry).collect();
        RedirectTable::from_entries(entries).map_err(StoreRefusal::Table)
    }

    /// Stores `entries`, made by `origin` now, once they are checked
    /// together with every stored redirect, so none takes a used address
    /// or closes a loop. The Admin API and the command line both store
    /// through here.
    ///
    /// # Errors
    ///
    /// [`StoreRefusal::Table`] for a loop or an address given twice,
    /// [`StoreRefusal::Taken`] for an address that already has a rule,
    /// and [`StoreRefusal::Storage`] when the store fails.
    pub async fn store_checked(
        &self,
        entries: Vec<RedirectEntry>,
        origin: Origin,
    ) -> Result<Vec<StoredRedirect>, StoreRefusal> {
        let stored = self.store.list().await?;
        let all = stored
            .into_iter()
            .map(|s| s.entry)
            .chain(entries.iter().cloned());
        RedirectTable::from_entries(all.collect()).map_err(StoreRefusal::Table)?;
        Ok(self.store.add(entries, origin, self.clock.now()).await?)
    }

    /// The desk over a site's stores, pages and clock.
    pub fn new(
        store: Arc<dyn RedirectStore>,
        log: Arc<dyn NotFoundLog>,
        pages: Arc<dyn PageAddresses>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            store,
            log,
            pages,
            clock,
        }
    }

    /// A desk for a router with no database behind it: no redirects, an
    /// empty log, and every write refused.
    pub fn none() -> Self {
        let store: Arc<dyn RedirectStore> = Arc::new(Nothing);
        let log: Arc<dyn NotFoundLog> = Arc::new(Nothing);
        let pages = Arc::new(StaticPages::default());
        Self::new(store, log, pages, Arc::new(SystemClock))
    }
}

impl std::fmt::Debug for RedirectDesk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedirectDesk").finish_non_exhaustive()
    }
}

/// Holds nothing and refuses every write.
#[derive(Debug)]
struct Nothing;

/// Why [`Nothing`] refuses a write.
#[derive(Debug, thiserror::Error)]
#[error("this server keeps no redirects")]
struct NoStore;

impl RedirectStore for Nothing {
    fn list(&self) -> BoxFuture<'_, Result<Vec<StoredRedirect>, RedirectStoreError>> {
        Box::pin(std::future::ready(Ok(Vec::new())))
    }

    fn add(
        &self,
        _: Vec<RedirectEntry>,
        _: Origin,
        _: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Vec<StoredRedirect>, RedirectStoreError>> {
        Box::pin(std::future::ready(Err(RedirectStoreError::storage(
            NoStore,
        ))))
    }

    fn remove(&self, _: i64) -> BoxFuture<'_, Result<bool, RedirectStoreError>> {
        Box::pin(std::future::ready(Ok(false)))
    }
}

impl NotFoundLog for Nothing {
    fn record<'a>(
        &'a self,
        _: &'a str,
        _: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), NotFoundLogError>> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn misses(&self, _: u32) -> BoxFuture<'_, Result<Vec<Miss>, NotFoundLogError>> {
        Box::pin(std::future::ready(Ok(Vec::new())))
    }

    fn forget<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<bool, NotFoundLogError>> {
        Box::pin(std::future::ready(Ok(false)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_desk_with_no_database_holds_nothing_and_refuses_writes() {
        let desk = RedirectDesk::none();
        assert_eq!(desk.store.list().await.unwrap(), []);
        let refused = desk.store.add(Vec::new(), Origin::Manual, desk.clock.now());
        assert!(matches!(refused.await, Err(RedirectStoreError::Storage(_))));
        assert!(!desk.store.remove(1).await.unwrap());
        desk.log
            .record("/blog/old/", desk.clock.now())
            .await
            .unwrap();
        assert_eq!(desk.log.misses(10).await.unwrap(), []);
        assert!(!desk.log.forget("/blog/old/").await.unwrap());
        assert_eq!(desk.pages.addresses().await.unwrap(), []);
        assert_eq!(format!("{desk:?}"), "RedirectDesk { .. }");
    }
}
