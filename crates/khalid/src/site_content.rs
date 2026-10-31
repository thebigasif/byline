//! What the Content API answers with on a running site: the published posts
//! of the content index and the site's lists, held in memory without their
//! bodies. A request renders the bodies of the posts it returns from their
//! files, so the store is small and quick to build.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use khalid_api::content_store::{
    BoxFuture, ContentStore, ContentStoreError, Listed, MemoryContentStore, PostKey, SiteResources,
};
use khalid_api::query::ListQuery;
use khalid_api::resources::{Bodies, Collection, Post};
use khalid_content::{SiteContent, parse_post, render};
use khalid_core::{AbsoluteUrl, Config, Locale, SitePath, SlugMode, UserMessage};
use khalid_db::{ContentIndex, IndexedPost, PostChanges};
use tokio::sync::watch;

use crate::entry_events::Entries;
use crate::post_lock::Lock;
use crate::tag_events::Tags;
use crate::{build_message, build_pages, index, site_lists};

/// What one render of the pages read for the Content API, without a single
/// post body: the content index, which posts are featured, and the lists.
#[derive(Debug, Clone, Default)]
pub(crate) struct Catalog {
    /// The rows a rebuild writes to `data/site.db`.
    pub(crate) index: ContentIndex,
    /// The paths of the posts marked `featured`.
    featured: BTreeSet<String>,
    /// The `updated` date of each post that has one, by path, so the
    /// Content API says when a post last really changed.
    updated: BTreeMap<String, NaiveDate>,
    lists: SiteResources,
    /// The pages, glossary terms, case studies and changelog entries.
    pub(crate) entries: Entries,
    /// The tags the posts name.
    pub(crate) tags: Tags,
}

impl Catalog {
    /// The catalog of `content`, already read from the site `folder`.
    ///
    /// # Errors
    ///
    /// Returns what, why and fix when a post cannot be indexed or an
    /// address cannot be made from the site's settings.
    pub(crate) fn of(
        folder: &Path,
        config: &Config,
        content: &SiteContent,
    ) -> Result<Self, UserMessage> {
        let index = index::of_content(folder, content)?;
        let featured = content
            .posts
            .iter()
            .filter(|doc| doc.front.featured)
            .filter_map(|doc| index::slashed(&doc.path))
            .collect();
        let updated = content
            .posts
            .iter()
            .filter_map(|doc| Some((index::slashed(&doc.path)?, doc.front.updated?)))
            .collect();
        let lists = site_lists::of(config, &index, content)?;
        Ok(Self {
            index,
            featured,
            updated,
            lists,
            entries: Entries::of(content),
            tags: Tags::of(config, content),
        })
    }
}

/// The published content readers see now. Cloning shares it. Until the
/// first store is ready a request waits for it, and a refresh swaps the
/// store whole, so a request never sees half a rebuild.
#[derive(Debug, Clone)]
pub(crate) struct LiveContent(Arc<watch::Sender<State>>);

#[derive(Debug, Clone)]
enum State {
    Loading,
    Ready(Arc<Published>),
    Failed,
}

impl Default for LiveContent {
    fn default() -> Self {
        Self(Arc::new(watch::channel(State::Loading).0))
    }
}

impl LiveContent {
    /// Serves `published` until it is replaced.
    pub(crate) fn replace(&self, published: impl Into<Arc<Published>>) {
        self.0.send_replace(State::Ready(published.into()));
    }

    /// Answers waiting requests with an error when no store was ever ready.
    /// A store already served stays.
    pub(crate) fn fail(&self) {
        self.0.send_if_modified(|state| {
            let loading = matches!(state, State::Loading);
            if loading {
                *state = State::Failed;
            }
            loading
        });
    }

    async fn current(&self) -> Result<Arc<Published>, ContentStoreError> {
        let mut states = self.0.subscribe();
        let state = states
            .wait_for(|state| !matches!(state, State::Loading))
            .await
            .map_err(|_| ContentStoreError::new(NotLoaded))?;
        match &*state {
            State::Ready(published) => Ok(Arc::clone(published)),
            State::Loading | State::Failed => Err(ContentStoreError::new(NotLoaded)),
        }
    }
}

impl ContentStore for LiveContent {
    fn list<'a>(
        &'a self,
        collection: Collection,
        query: &'a ListQuery,
    ) -> BoxFuture<'a, Result<Listed<Post>, ContentStoreError>> {
        Box::pin(async move {
            let published = self.current().await?;
            let listed = published.store.list(collection, query).await?;
            if query.formats().is_empty() {
                return Ok(listed);
            }
            let total = listed.total();
            let posts = with_bodies(published, listed.into_items()).await?;
            Ok(Listed::new(posts, total))
        })
    }

    fn find<'a>(
        &'a self,
        collection: Collection,
        key: &'a PostKey,
    ) -> BoxFuture<'a, Result<Option<Post>, ContentStoreError>> {
        Box::pin(async move {
            let published = self.current().await?;
            let Some(post) = published.store.find(collection, key).await? else {
                return Ok(None);
            };
            Ok(with_bodies(published, vec![post]).await?.pop())
        })
    }

    fn site(&self) -> BoxFuture<'_, Result<SiteResources, ContentStoreError>> {
        Box::pin(async move { self.current().await?.store.site().await })
    }
}

/// The site's published content could not be read from its files, so the
/// Content API has nothing to answer with until the next save.
#[derive(Debug)]
struct NotLoaded;

impl fmt::Display for NotLoaded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the published posts could not be read from the site's files")
    }
}

impl std::error::Error for NotLoaded {}

/// One store of published posts, without bodies, and where each body is.
#[derive(Debug)]
pub(crate) struct Published {
    store: MemoryContentStore,
    /// Each published post, by the path of its file in the index.
    posts: BTreeMap<String, Listing>,
    /// The path of each post's file, by the post's identity.
    files: BTreeMap<String, String>,
    folder: PathBuf,
    slugs: SlugMode,
}

/// One published post, and the language and day that order it.
#[derive(Debug, Clone)]
struct Listing {
    post: Post,
    locale: Locale,
    date: NaiveDate,
}

impl Published {
    /// The published posts of `catalog`, read from the site `folder` with
    /// its `config`, in every language. A post is published when it has a
    /// date, as on the public site.
    ///
    /// # Errors
    ///
    /// Returns what, why and fix when an address cannot be made from the
    /// site's settings.
    pub(crate) fn of(
        folder: &Path,
        config: &Config,
        catalog: &Catalog,
    ) -> Result<Self, UserMessage> {
        let posts = listings(config, catalog, catalog.index.posts.iter())?;
        Ok(Self::assemble(
            folder,
            config,
            posts.into_iter().collect(),
            catalog,
        ))
    }

    /// This store after a save: the posts `changes` names are made again
    /// from `catalog` or removed, every other post is kept as it is, and
    /// the lists come from `catalog`.
    ///
    /// # Errors
    ///
    /// Returns what, why and fix when an address cannot be made from the
    /// site's settings.
    pub(crate) fn changed(
        &self,
        config: &Config,
        catalog: &Catalog,
        changes: &PostChanges,
    ) -> Result<Self, UserMessage> {
        let mut posts = self.posts.clone();
        for path in changes.written.iter().chain(&changes.removed) {
            posts.remove(path);
        }
        let written = catalog
            .index
            .posts
            .iter()
            .filter(|row| changes.written.contains(&row.path));
        posts.extend(listings(config, catalog, written)?);
        Ok(Self::assemble(&self.folder, config, posts, catalog))
    }

    /// The store of `posts`, in each language of `config` newest first, and
    /// posts of one day in slug order, as the index lists them.
    fn assemble(
        folder: &Path,
        config: &Config,
        posts: BTreeMap<String, Listing>,
        catalog: &Catalog,
    ) -> Self {
        let mut entries = Vec::with_capacity(posts.len());
        let mut files = BTreeMap::new();
        for locale in config.languages() {
            let mut rows: Vec<(&String, &Listing)> = posts
                .iter()
                .filter(|(_, listing)| &listing.locale == locale)
                .collect();
            rows.sort_by(|(_, a), (_, b)| {
                let (a_slug, b_slug) = (a.post.slug().as_str(), b.post.slug().as_str());
                b.date.cmp(&a.date).then_with(|| a_slug.cmp(b_slug))
            });
            for (path, listing) in rows {
                let id = listing.post.id().to_owned();
                files.entry(id).or_insert_with(|| path.clone());
                entries.push((Collection::Posts, listing.post.clone()));
            }
        }
        Self {
            store: MemoryContentStore::new(entries).with_site(catalog.lists.clone()),
            posts,
            files,
            folder: folder.to_path_buf(),
            slugs: config.slugs(),
        }
    }

    /// The body of the post with identity `id` in every format, read from
    /// its file and rendered now.
    fn bodies(&self, id: &str) -> Result<Bodies, ContentStoreError> {
        let Some((path, listing)) = self
            .files
            .get(id)
            .and_then(|path| Some((path, self.posts.get(path)?)))
        else {
            return Ok(Bodies::default());
        };
        let locale = &listing.locale;
        let file = SitePath::within(&self.folder, path).map_err(ContentStoreError::new)?;
        let text = std::fs::read_to_string(file.as_path()).map_err(ContentStoreError::new)?;
        let post = parse_post(file.relative(), &text).map_err(ContentStoreError::new)?;
        // A locked post's body stays on the server, as on its page.
        if Lock::of(&post.front).is_some() {
            return Ok(Bodies::default());
        }
        let body = post.body;
        let rendered = render(&body, locale, self.slugs).map_err(ContentStoreError::new)?;
        Ok(Bodies::new(
            &rendered.html,
            &body,
            &plaintext(&rendered.html),
        ))
    }
}

/// `posts` with their bodies, rendered on a blocking thread, since each
/// one reads a file (RULES.md BL-RUN-7).
async fn with_bodies(
    published: Arc<Published>,
    posts: Vec<Post>,
) -> Result<Vec<Post>, ContentStoreError> {
    let work = move || {
        posts
            .into_iter()
            .map(|post| {
                let bodies = published.bodies(post.id())?;
                Ok(post.with_bodies(bodies))
            })
            .collect()
    };
    tokio::task::spawn_blocking(work)
        .await
        .map_err(ContentStoreError::new)?
}

/// The published posts among `rows` of the index of `catalog`, by path.
fn listings<'a>(
    config: &Config,
    catalog: &Catalog,
    rows: impl Iterator<Item = &'a IndexedPost>,
) -> Result<Vec<(String, Listing)>, UserMessage> {
    let mut found = Vec::new();
    for row in rows {
        let Some(date) = row.date else {
            continue;
        };
        let mut post = post_of(config, row, date)?;
        if catalog.featured.contains(&row.path) {
            post = post.featured();
        }
        if let Some(day) = catalog.updated.get(&row.path).filter(|day| **day > date) {
            post = post.with_updated_at(midnight(*day));
        }
        let locale = row.locale.clone();
        found.push((row.path.clone(), Listing { post, locale, date }));
    }
    Ok(found)
}

/// The index `row` of a post published on `date`, in API shape, without
/// its body.
fn post_of(config: &Config, row: &IndexedPost, date: NaiveDate) -> Result<Post, UserMessage> {
    let locale = &row.locale;
    let path = build_pages::path_of(config, locale, &format!("{}/", row.slug));
    let address = format!("{}{path}", config.address().as_str().trim_end_matches('/'));
    let url = AbsoluteUrl::parse(&address).map_err(|_| build_message::address(&address))?;
    let at = midnight(date);
    let id = format!("{}.{}", locale.as_str(), row.slug);
    let title = row.title.clone().unwrap_or_else(|| row.slug.to_string());
    let mut post = Post::new(&id, row.slug.clone(), &title, url, locale.clone(), at)
        .with_authors(row.author.iter().map(ToString::to_string).collect());
    if let Some(excerpt) = &row.description {
        post = post.with_excerpt(excerpt);
    }
    Ok(post)
}

/// The first second of `day`, in UTC.
fn midnight(day: NaiveDate) -> DateTime<Utc> {
    DateTime::from_naive_utc_and_offset(day.and_time(NaiveTime::MIN), Utc)
}

/// `html` with its tags removed, the common entities read back, and runs
/// of white space made one space.
fn plaintext(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    let text = text
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use khalid_api::query::RawListQuery;
    use khalid_content::{ContentTree, read_content};
    use khalid_core::Environment;

    use super::*;

    fn demo() -> (PathBuf, Config, Catalog) {
        let mut folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        folder.push("../../fixtures/demo-site");
        let config = Config::load(&folder, &Environment::empty()).unwrap();
        let content = read_content(&ContentTree::walk(&folder).unwrap()).unwrap();
        let catalog = Catalog::of(&folder, &config, &content).unwrap();
        (folder, config, catalog)
    }

    fn english() -> ListQuery {
        let mut raw = RawListQuery::default();
        raw.filter = Some("language:en".to_owned());
        ListQuery::try_from(&raw).unwrap()
    }

    #[tokio::test]
    async fn the_demo_posts_come_back_from_the_index_with_their_bodies() {
        let (folder, config, catalog) = demo();
        let live = LiveContent::default();
        live.replace(Published::of(&folder, &config, &catalog).unwrap());

        let listed = live.list(Collection::Posts, &english()).await.unwrap();
        let json = serde_json::to_value(listed.items()).unwrap();
        assert_eq!(listed.total(), 5, "{json}");
        let first = &json[0];
        assert_eq!(first["id"], "en.planning-a-topic-cluster");
        assert_eq!(
            first["url"],
            "https://www.example.com/blog/planning-a-topic-cluster/"
        );
        assert_eq!(first["published_at"], "2026-10-12T00:00:00Z");
        assert!(first["html"].as_str().unwrap().contains("<p>"), "{first}");
        assert!(
            !first["plaintext"].as_str().unwrap().contains('<'),
            "{first}"
        );

        let german = PostKey::Id("de.warum-interne-links-wichtig-sind".to_owned());
        let found = live
            .find(Collection::Posts, &german)
            .await
            .unwrap()
            .unwrap();
        let found = serde_json::to_value(found).unwrap();
        assert_eq!(
            found["url"],
            "https://www.example.com/blog/de/warum-interne-links-wichtig-sind/"
        );
        assert!(found["markdown"].as_str().unwrap().len() > 10, "{found}");
    }

    #[tokio::test]
    async fn the_store_holds_no_body_until_one_is_asked_for() {
        let (folder, config, catalog) = demo();
        let published = Published::of(&folder, &config, &catalog).unwrap();
        let held = published.store.list(Collection::Posts, &english()).await;
        let held = serde_json::to_value(held.unwrap().items()).unwrap();
        assert!(held[0].get("html").is_none(), "{held}");
        assert!(held[0].get("markdown").is_none(), "{held}");
    }

    #[tokio::test]
    async fn a_change_makes_only_the_posts_it_names_again() {
        let (folder, config, mut catalog) = demo();
        let before = Published::of(&folder, &config, &catalog).unwrap();
        let mut changes = PostChanges::default();
        for row in &mut catalog.index.posts {
            row.title = Some(format!("Retitled {}", row.slug));
        }
        let edited = "content/en/posts/2026-09-28-what-a-sitemap-does.md";
        changes.written.insert(edited.to_owned());
        changes
            .removed
            .insert("content/en/posts/2026-10-12-planning-a-topic-cluster.md".to_owned());
        catalog
            .index
            .posts
            .retain(|row| !row.path.contains("planning"));
        let after = before.changed(&config, &catalog, &changes).unwrap();

        let listed = after.store.list(Collection::Posts, &english()).await;
        let titles: Vec<String> = serde_json::to_value(listed.unwrap().items())
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|post| post["title"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(titles.len(), 4, "{titles:?}");
        let retitled: Vec<&String> = titles
            .iter()
            .filter(|t| t.starts_with("Retitled"))
            .collect();
        assert_eq!(retitled, ["Retitled what-a-sitemap-does"], "{titles:?}");
    }

    #[tokio::test]
    async fn a_body_whose_file_is_gone_is_an_error_not_an_empty_post() {
        let (_, config, catalog) = demo();
        let elsewhere = tempfile::tempdir().unwrap();
        let live = LiveContent::default();
        live.replace(Published::of(elsewhere.path(), &config, &catalog).unwrap());
        assert!(live.list(Collection::Posts, &english()).await.is_err());
        let german = PostKey::Id("de.warum-interne-links-wichtig-sind".to_owned());
        assert!(live.find(Collection::Posts, &german).await.is_err());
    }

    #[tokio::test]
    async fn a_request_waits_for_the_first_store_instead_of_answering_empty() {
        let (folder, config, catalog) = demo();
        let live = LiveContent::default();
        let asking = live.clone();
        let answer = tokio::spawn(async move {
            let listed = asking.list(Collection::Posts, &english()).await;
            listed.map(|listed| listed.total())
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!answer.is_finished(), "answered before the store was ready");
        live.replace(Published::of(&folder, &config, &catalog).unwrap());
        assert_eq!(answer.await.unwrap().unwrap(), 5);
    }

    #[tokio::test]
    async fn a_store_that_never_loaded_answers_an_error_but_a_served_one_stays() {
        let (folder, config, catalog) = demo();
        let live = LiveContent::default();
        live.fail();
        assert!(live.list(Collection::Posts, &english()).await.is_err());
        live.replace(Published::of(&folder, &config, &catalog).unwrap());
        live.fail();
        let listed = live.list(Collection::Posts, &english()).await.unwrap();
        assert_eq!(listed.total(), 5);
    }

    #[test]
    fn plaintext_drops_tags_and_reads_entities() {
        assert_eq!(
            plaintext("<p>Links &amp; <a href=\"/x/\">anchors</a></p>\n<p>&lt;3</p>"),
            "Links & anchors <3"
        );
    }
}
