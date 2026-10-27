//! The stored redirects `khalid serve` answers (KH-SEO-8): the rows in
//! `data/site.db`, with the checked table held in memory so a request
//! never waits for the database. Every write through the Admin API
//! reloads the table, so a new redirect answers at once.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use khalid_api::content_store::BoxFuture;
use khalid_api::redirect_store::{Origin, RedirectStore, RedirectStoreError, StoredRedirect};
use khalid_seo::{RedirectEntry, RedirectTable};
use tokio::sync::watch;

/// The site's stored redirects and the table they make. Cloning shares
/// both.
#[derive(Clone)]
pub(crate) struct LiveRedirects {
    store: Arc<dyn RedirectStore>,
    table: Arc<watch::Sender<Arc<RedirectTable>>>,
}

impl LiveRedirects {
    /// The redirects in `store`, serving no rules until the first
    /// [`LiveRedirects::reload`].
    pub(crate) fn new(store: Arc<dyn RedirectStore>) -> Self {
        Self {
            store,
            table: Arc::new(watch::channel(Arc::new(RedirectTable::default())).0),
        }
    }

    /// The table being served now.
    pub(crate) fn table(&self) -> Arc<RedirectTable> {
        Arc::clone(&self.table.borrow())
    }

    /// Reads the rows again and serves the table they make. Rows that
    /// cannot be read, or that make no table, such as two writers racing
    /// into a loop, leave the last table served, and the log says why.
    pub(crate) async fn reload(&self) {
        let entries = match self.store.list().await {
            Ok(stored) => stored.into_iter().map(|s| s.entry).collect(),
            Err(error) => {
                tracing::warn!(%error, "the redirects keep their last table");
                return;
            }
        };
        match RedirectTable::from_entries(entries) {
            Ok(table) => {
                self.table.send_replace(Arc::new(table));
            }
            Err(error) => tracing::warn!(%error, "the redirects keep their last table"),
        }
    }
}

impl std::fmt::Debug for LiveRedirects {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveRedirects").finish_non_exhaustive()
    }
}

/// The store's own answers, with the table reloaded after each write.
impl RedirectStore for LiveRedirects {
    fn list(&self) -> BoxFuture<'_, Result<Vec<StoredRedirect>, RedirectStoreError>> {
        self.store.list()
    }

    fn add(
        &self,
        entries: Vec<RedirectEntry>,
        origin: Origin,
        at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Vec<StoredRedirect>, RedirectStoreError>> {
        Box::pin(async move {
            let added = self.store.add(entries, origin, at).await?;
            self.reload().await;
            Ok(added)
        })
    }

    fn remove(&self, id: i64) -> BoxFuture<'_, Result<bool, RedirectStoreError>> {
        Box::pin(async move {
            let removed = self.store.remove(id).await?;
            self.reload().await;
            Ok(removed)
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use khalid_api::redirect_store::MemoryRedirectStore;
    use khalid_seo::{UrlPath, read_redirects_csv};

    use super::*;

    fn at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap()
    }

    fn entries(rows: &str) -> Vec<RedirectEntry> {
        read_redirects_csv(&format!("from,to,status\n{rows}")).unwrap()
    }

    fn status(live: &LiveRedirects, path: &str) -> Option<u16> {
        let path = UrlPath::try_new(path).unwrap();
        live.table().resolve(&path).map(|hit| hit.status())
    }

    #[tokio::test]
    async fn the_table_follows_every_write_and_keeps_the_last_good_one() {
        let store = MemoryRedirectStore::default();
        store
            .add(entries("/blog/a/,/blog/b/,301\n"), Origin::Manual, at())
            .await
            .unwrap();
        let store = Arc::new(store);
        let shared: Arc<dyn RedirectStore> = store;
        let live = LiveRedirects::new(shared);
        live.reload().await;
        assert_eq!(status(&live, "/blog/a/"), Some(301));

        let added = live
            .add(entries("/blog/c/,,410\n"), Origin::Manual, at())
            .await
            .unwrap();
        assert_eq!(status(&live, "/blog/c/"), Some(410));
        assert!(live.remove(added[0].id).await.unwrap());
        assert_eq!(status(&live, "/blog/c/"), None);

        // A loop written past the checks, as two racing writers could.
        live.add(entries("/blog/b/,/blog/a/,301\n"), Origin::Manual, at())
            .await
            .unwrap();
        assert_eq!(status(&live, "/blog/a/"), Some(301));
        assert_eq!(status(&live, "/blog/b/"), None);
        assert_eq!(live.list().await.unwrap().len(), 2);
    }
}
