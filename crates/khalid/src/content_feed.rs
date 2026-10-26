//! The background task that turns each render's catalog into the Content
//! API's store and the index in `data/site.db`, so pages never wait for
//! either (`docs/decisions/2026-10-27-cto-answers.md`, section 3).

use std::path::PathBuf;
use std::sync::Arc;

use khalid_core::Config;
use khalid_db::{Database, SiteId};
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::db_message;
use crate::site_content::{Catalog, LiveContent, Published};

/// Hands the catalog of each render to the background task. When every
/// feed is dropped, the task finishes what it is doing and stops.
#[derive(Debug)]
pub(crate) struct ContentFeed(mpsc::UnboundedSender<Catalog>);

impl ContentFeed {
    /// Queues `catalog` and returns at once. Of several catalogs queued
    /// while one is built, only the last is built next.
    pub(crate) fn send(&self, catalog: Catalog) {
        // The task is gone only once the server stops, when nothing is left
        // to show the catalog to.
        if self.0.send(catalog).is_err() {
            tracing::debug!("the Content API's task has stopped");
        }
    }
}

/// Where the catalogs go: the site's database and the live store.
#[derive(Debug, Clone)]
pub(crate) struct Destination {
    pub(crate) folder: PathBuf,
    pub(crate) config: Config,
    pub(crate) db: Database,
    pub(crate) site: SiteId,
    pub(crate) live: LiveContent,
}

/// Starts the background task on `runtime`. Await the handle after the
/// last feed is dropped and before the database closes.
pub(crate) fn start(runtime: &Handle, to: Destination) -> (ContentFeed, JoinHandle<()>) {
    let (feed, catalogs) = mpsc::unbounded_channel();
    let task = runtime.spawn(take_catalogs(catalogs, Arc::new(to)));
    (ContentFeed(feed), task)
}

/// Builds the latest catalog each time one arrives, and drops it once it is
/// served and written, so no catalog stays in memory.
async fn take_catalogs(mut catalogs: mpsc::UnboundedReceiver<Catalog>, to: Arc<Destination>) {
    while let Some(mut catalog) = catalogs.recv().await {
        while let Ok(later) = catalogs.try_recv() {
            catalog = later;
        }
        apply(&to, Arc::new(catalog)).await;
    }
}

/// Serves the store built from `catalog`, then writes its index. A store
/// that cannot be built leaves the last good one in place, and an index
/// that cannot be written leaves the store as it is.
async fn apply(to: &Arc<Destination>, catalog: Arc<Catalog>) {
    let building = {
        let (to, catalog) = (Arc::clone(to), Arc::clone(&catalog));
        tokio::task::spawn_blocking(move || Published::of(&to.folder, &to.config, &catalog))
    };
    match building.await {
        Ok(Ok(published)) => to.live.replace(published),
        Ok(Err(message)) => {
            tracing::warn!(%message, "the Content API keeps its last posts");
            to.live.fail();
        }
        Err(error) => {
            tracing::warn!(?error, "the Content API keeps its last posts");
            to.live.fail();
        }
    }
    if let Err(error) = to.db.rebuild(to.site, &catalog.index).await {
        let message = db_message::of(&to.folder, &error);
        tracing::warn!(%message, "the index in data/site.db keeps its last rows");
    }
}

#[cfg(test)]
mod tests {
    use khalid_api::content_store::ContentStore;
    use khalid_api::query::{ListQuery, RawListQuery};
    use khalid_api::resources::Collection;
    use khalid_content::{ContentTree, read_content};
    use khalid_core::{Environment, Slug};

    use super::*;

    #[tokio::test]
    async fn a_sent_catalog_is_served_and_written_to_the_index() {
        let mut folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        folder.push("../../fixtures/demo-site");
        let config = Config::load(&folder, &Environment::empty()).unwrap();
        let content = read_content(&ContentTree::walk(&folder).unwrap()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let mut file = dir.path().to_path_buf();
        file.push("site.db");
        let db = Database::open(&file).await.unwrap();
        db.migrate().await.unwrap();
        let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
        let live = LiveContent::default();
        let to = Destination {
            folder: folder.clone(),
            config: config.clone(),
            db: db.clone(),
            site,
            live: live.clone(),
        };
        let (feed, task) = start(&Handle::current(), to);

        feed.send(Catalog::of(&folder, &config, &content).unwrap());
        let query = ListQuery::try_from(&RawListQuery::default()).unwrap();
        let listed = live.list(Collection::Posts, &query).await.unwrap();
        assert!(listed.total() > 5, "{} posts", listed.total());
        drop(feed);
        task.await.unwrap();

        let english = config.default_language();
        let rows = db.posts(site, english, "posts", u32::MAX).await.unwrap();
        assert_eq!(rows.len(), 5, "the five English demo posts");
        db.close().await;
    }
}
