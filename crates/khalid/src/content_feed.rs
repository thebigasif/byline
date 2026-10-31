//! The background task that turns each render's catalog into the Content
//! API's store and the index in `data/site.db`, with its search index, so
//! pages never wait for either (`docs/decisions/2026-10-27-cto-answers.md`,
//! section 3).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use chrono::NaiveDate;
use khalid_core::{AbsoluteUrl, Clock, Config, SystemClock};
use khalid_db::{Database, PostChanges, SiteId};
use khalid_seo::UrlPath;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::announce::SiteAnnounce;
use crate::entry_events::Entries;
use crate::live_redirects::LiveRedirects;
use crate::memory::HandBack;
use crate::site_addresses::{self, LiveAddresses};
use crate::site_content::{Catalog, LiveContent, Published};
use crate::tag_events::Tags;
use crate::{db_message, search_index, slug_moves};

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
    /// Told after each catalog is served and written.
    pub(crate) hand_back: HandBack,
    /// The address of each page, for the redirect routes.
    pub(crate) addresses: LiveAddresses,
    /// The stored redirects, which get a move for each page that moved.
    pub(crate) redirects: LiveRedirects,
    /// Told of each page, glossary term, case study or changelog entry that
    /// is new, changed or gone since the last catalog, when the site has
    /// webhooks to tell.
    pub(crate) announce: Option<SiteAnnounce>,
    /// The entries and tags the run started with, which the first catalog
    /// is compared with.
    pub(crate) started: (Entries, Tags),
    /// Where the addresses of live posts with news go, for `IndexNow`.
    pub(crate) pings: UpdatePings,
}

/// The announcer that sends `IndexNow` the addresses of live posts whose
/// files show news, once `khalid serve` has started it. Until then, and in
/// a run with no announcer, nothing is sent. Cloning shares it.
#[derive(Debug, Clone, Default)]
pub(crate) struct UpdatePings(Arc<OnceLock<SiteAnnounce>>);

impl UpdatePings {
    /// Sends to `telling` from now on. Only the first call counts.
    pub(crate) fn set(&self, telling: SiteAnnounce) {
        if self.0.set(telling).is_err() {
            tracing::debug!("the IndexNow announcer was already set");
        }
    }

    /// Sends the full addresses of `paths` on the site with `config`.
    fn send(&self, config: &Config, paths: &[&UrlPath]) {
        let Some(telling) = self.0.get() else {
            return;
        };
        let base = config.address().as_str().trim_end_matches('/');
        let urls: Vec<AbsoluteUrl> = paths
            .iter()
            .filter_map(|path| AbsoluteUrl::parse(&format!("{base}{}", path.as_str())).ok())
            .collect();
        if !urls.is_empty() {
            telling.updated(urls);
        }
    }
}

/// Each live post's address and `updated:` date, by the path of its file.
#[derive(Debug, Default)]
struct Live {
    addresses: BTreeMap<String, UrlPath>,
    updated: BTreeMap<String, NaiveDate>,
}

/// The addresses of the posts in `now` that went live since `before`, or
/// whose `updated:` date is new or later. The date changes only on a real
/// update (decision D4), so any other edit tells nothing.
fn told<'a>(before: &Live, now: &'a Live) -> Vec<&'a UrlPath> {
    now.addresses
        .iter()
        .filter(|(file, _)| {
            !before.addresses.contains_key(*file)
                || now.updated.get(*file) > before.updated.get(*file)
        })
        .map(|(_, path)| path)
        .collect()
}

/// Starts the background task on `runtime`. Await the handle after the
/// last feed is dropped and before the database closes.
pub(crate) fn start(runtime: &Handle, to: Destination) -> (ContentFeed, JoinHandle<()>) {
    let (feed, catalogs) = mpsc::unbounded_channel();
    let task = runtime.spawn(take_catalogs(catalogs, Arc::new(to)));
    (ContentFeed(feed), task)
}

/// The content hash of each post, by the path of its file in the index.
type Hashes = BTreeMap<String, [u8; 32]>;

/// What the task last served and wrote, which the next catalog is compared
/// with. A hash list is `None` when that side must be built whole next.
#[derive(Debug, Default)]
struct Applied {
    published: Option<Arc<Published>>,
    served: Option<Arc<Hashes>>,
    written: Option<Arc<Hashes>>,
    /// Each page's address in the last catalog, by file.
    addresses: Option<BTreeMap<String, UrlPath>>,
    /// The live posts of the last catalog; `None` before the first, which
    /// has nothing to compare with and tells nothing.
    live: Option<Live>,
}

/// Serves the addresses of `catalog`'s pages, and stores a permanent move
/// for each page whose address changed since the last catalog, or, for
/// the first, since the index this run started with.
async fn moved(to: &Destination, catalog: &Catalog, applied: &mut Applied) {
    let after = site_addresses::of(&to.config, &catalog.index);
    let before = match applied.addresses.take() {
        Some(before) => before,
        None => slug_moves::indexed((&to.db, to.site), &to.config, &catalog.index).await,
    };
    slug_moves::store(&to.redirects, (&before, &after), SystemClock.now()).await;
    to.addresses.replace(&after);
    let now = Live {
        addresses: after.clone(),
        updated: catalog.updated.clone(),
    };
    if let Some(before) = &applied.live {
        to.pings.send(&to.config, &told(before, &now));
    }
    applied.live = Some(now);
    applied.addresses = Some(after);
}

/// Builds the latest catalog each time one arrives, and drops it once it is
/// served and written, so no catalog stays in memory.
async fn take_catalogs(mut catalogs: mpsc::UnboundedReceiver<Catalog>, to: Arc<Destination>) {
    let mut applied = Applied::default();
    let (mut entries, mut tags) = to.started.clone();
    while let Some(mut catalog) = catalogs.recv().await {
        while let Ok(later) = catalogs.try_recv() {
            catalog = later;
        }
        let now = std::mem::take(&mut catalog.entries);
        let tagged = std::mem::take(&mut catalog.tags);
        if let Some(announce) = &to.announce {
            now.fire_since(&entries, announce);
            tagged.fire_since(&tags, announce);
        }
        (entries, tags) = (now, tagged);
        apply(&to, Arc::new(catalog), &mut applied).await;
        to.hand_back.after_work();
    }
}

/// Serves the store for `catalog`, then writes its index. After the first
/// time, only the posts whose files changed are made again on each side. A
/// store that cannot be built leaves the last good one in place, an index
/// that cannot be written leaves the store as it is, and either is built
/// whole the next time.
async fn apply(to: &Arc<Destination>, catalog: Arc<Catalog>, applied: &mut Applied) {
    let hashes: Hashes = catalog
        .index
        .posts
        .iter()
        .map(|post| (post.path.clone(), post.content_hash))
        .collect();
    let hashes = Arc::new(hashes);
    moved(to, &catalog, applied).await;
    let since = applied.published.clone().zip(applied.served.as_ref());
    let since = since.map(|(published, served)| (published, changes(served, &hashes)));
    let building = {
        let (to, catalog) = (Arc::clone(to), Arc::clone(&catalog));
        tokio::task::spawn_blocking(move || match since {
            Some((published, changes)) => published.changed(&to.config, &catalog, &changes),
            None => Published::of(&to.folder, &to.config, &catalog),
        })
    };
    match building.await {
        Ok(Ok(published)) => {
            let published = Arc::new(published);
            to.live.replace(Arc::clone(&published));
            applied.published = Some(published);
            applied.served = Some(Arc::clone(&hashes));
        }
        Ok(Err(message)) => kept(to, &mut applied.served, &message),
        Err(error) => kept(to, &mut applied.served, &error),
    }
    let index_changes = applied
        .written
        .as_ref()
        .map(|before| changes(before, &hashes));
    let written = match &index_changes {
        Some(changes) => to.db.update(to.site, &catalog.index, changes).await,
        None => to.db.rebuild(to.site, &catalog.index).await,
    };
    let which = match &index_changes {
        Some(changes) => search_index::Which::Changed {
            written: &changes.written,
            removed: &changes.removed,
        },
        None => search_index::Which::All,
    };
    let at = (&to.db, to.site);
    let written = match written {
        Ok(_) => search_index::write(at, &to.folder, &to.config, &catalog, which).await,
        Err(error) => Err(error),
    };
    match written {
        Ok(()) => applied.written = Some(hashes),
        Err(error) => {
            applied.written = None;
            let message = db_message::of(&to.folder, &error);
            tracing::warn!(%message, "the index in data/site.db keeps its last rows");
        }
    }
}

/// Logs why the store was not built, and has the next one built whole.
fn kept(to: &Destination, served: &mut Option<Arc<Hashes>>, why: &dyn std::fmt::Display) {
    tracing::warn!(%why, "the Content API keeps its last posts");
    to.live.fail();
    *served = None;
}

/// The posts that are new, edited or gone in `now` since `before`.
fn changes(before: &Hashes, now: &Hashes) -> PostChanges {
    let mut changes = PostChanges::default();
    for (path, hash) in now {
        if before.get(path) != Some(hash) {
            changes.written.insert(path.clone());
        }
    }
    for path in before.keys() {
        if !now.contains_key(path) {
            changes.removed.insert(path.clone());
        }
    }
    changes
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
    async fn a_sent_catalog_is_served_and_written_and_a_save_as_a_change() {
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
            hand_back: HandBack::default(),
            addresses: LiveAddresses::default(),
            redirects: LiveRedirects::new(Arc::new(khalid_db::RedirectRows::new(db.clone(), site))),
            announce: None,
            started: (Entries::default(), Tags::default()),
            pings: UpdatePings::default(),
        };
        let (feed, task) = start(&Handle::current(), to);

        feed.send(Catalog::of(&folder, &config, &content).unwrap());
        let query = ListQuery::try_from(&RawListQuery::default()).unwrap();
        let listed = live.list(Collection::Posts, &query).await.unwrap();
        assert!(listed.total() > 5, "{} posts", listed.total());

        // A save: one post has a new title, so a new hash.
        let mut saved = Catalog::of(&folder, &config, &content).unwrap();
        let edited = saved.index.posts.iter_mut().find(|row| {
            row.path
                .ends_with("en/posts/2026-09-28-what-a-sitemap-does.md")
        });
        let edited = edited.unwrap();
        edited.title = Some("What a sitemap is for".to_owned());
        edited.content_hash = [9; 32];
        feed.send(saved);
        drop(feed);
        task.await.unwrap();

        let english = config.default_language();
        let rows = db.posts(site, english, "posts", u32::MAX).await.unwrap();
        assert_eq!(rows.len(), 5, "the five English demo posts");
        let titles: Vec<_> = rows.iter().filter_map(|row| row.title.as_deref()).collect();
        assert!(titles.contains(&"What a sitemap is for"), "{titles:?}");
        let listed = live.list(Collection::Posts, &query).await.unwrap();
        let json = serde_json::to_value(listed.items()).unwrap().to_string();
        assert!(
            json.contains("What a sitemap is for"),
            "the store shows the save"
        );
        db.close().await;
    }

    #[test]
    fn a_new_updated_date_or_a_new_live_post_is_told_and_nothing_else() {
        let live = |posts: &[(&str, Option<u32>)]| {
            let mut live = Live::default();
            for &(file, day) in posts {
                let path = UrlPath::try_new(&format!("/blog/{file}/")).unwrap();
                live.addresses.insert(format!("{file}.md"), path);
                if let Some(day) = day.and_then(|day| NaiveDate::from_ymd_opt(2026, 10, day)) {
                    live.updated.insert(format!("{file}.md"), day);
                }
            }
            live
        };
        let before = live(&[
            ("dated", None),
            ("same", Some(1)),
            ("later", Some(5)),
            ("dropped", Some(5)),
        ]);
        let now = live(&[
            ("dated", Some(9)),
            ("same", Some(1)),
            ("later", Some(7)),
            ("dropped", None),
            ("new", None),
        ]);
        let told: Vec<&str> = told(&before, &now)
            .into_iter()
            .map(UrlPath::as_str)
            .collect();
        assert_eq!(told, ["/blog/dated/", "/blog/later/", "/blog/new/"]);
        assert!(
            super::told(&now, &now).is_empty(),
            "a save with no new date tells nothing"
        );
    }

    #[test]
    fn changes_name_new_edited_and_gone_posts_only() {
        let before: Hashes = [("a.md", 1), ("b.md", 2), ("c.md", 3)]
            .into_iter()
            .map(|(path, byte)| (path.to_owned(), [byte; 32]))
            .collect();
        let now: Hashes = [("a.md", 1), ("b.md", 7), ("d.md", 4)]
            .into_iter()
            .map(|(path, byte)| (path.to_owned(), [byte; 32]))
            .collect();
        let found = changes(&before, &now);
        let written: Vec<&str> = found.written.iter().map(String::as_str).collect();
        let removed: Vec<&str> = found.removed.iter().map(String::as_str).collect();
        assert_eq!(written, ["b.md", "d.md"]);
        assert_eq!(removed, ["c.md"]);
    }
}
