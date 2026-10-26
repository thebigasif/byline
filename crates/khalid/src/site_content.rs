//! What the Content API answers with on a running site: the published posts
//! of the content index and the site's lists, held in memory without their
//! bodies. A request renders the bodies of the posts it returns from their
//! files, so the store is small and quick to build.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock};

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use khalid_api::content_store::{
    BoxFuture, ContentStore, ContentStoreError, Listed, MemoryContentStore, PostKey, SiteResources,
};
use khalid_api::query::ListQuery;
use khalid_api::resources::{Bodies, Collection, Post};
use khalid_content::{SiteContent, parse_post, render};
use khalid_core::{AbsoluteUrl, Config, Locale, SitePath, SlugMode, UserMessage};
use khalid_db::{ContentIndex, Database, IndexedPost, SiteId};

use crate::{build_message, build_pages, db_message, index, site_lists};

/// What one render of the pages read for the Content API, without a single
/// post body: the content index, which posts are featured, and the lists.
#[derive(Debug, Clone, Default)]
pub(crate) struct Catalog {
    /// The rows a rebuild writes to `data/site.db`.
    pub(crate) index: ContentIndex,
    /// The paths of the posts marked `featured`.
    featured: BTreeSet<String>,
    lists: SiteResources,
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
        let lists = site_lists::of(config, &index, content)?;
        Ok(Self {
            index,
            featured,
            lists,
        })
    }
}

/// The published content readers see now. Cloning shares it, and a
/// refresh swaps it whole, so a request never sees half a rebuild.
#[derive(Debug, Clone, Default)]
pub(crate) struct LiveContent(Arc<RwLock<Arc<Published>>>);

impl LiveContent {
    /// Serves `published` until it is replaced.
    pub(crate) fn replace(&self, published: Published) {
        let mut slot = self.0.write().unwrap_or_else(PoisonError::into_inner);
        *slot = Arc::new(published);
    }

    fn current(&self) -> Arc<Published> {
        // Holding the lock only reads or swaps an `Arc`, so a poisoned lock
        // still holds a whole store.
        Arc::clone(&self.0.read().unwrap_or_else(PoisonError::into_inner))
    }
}

impl ContentStore for LiveContent {
    fn list<'a>(
        &'a self,
        collection: Collection,
        query: &'a ListQuery,
    ) -> BoxFuture<'a, Result<Listed<Post>, ContentStoreError>> {
        Box::pin(async move {
            let published = self.current();
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
            let published = self.current();
            let Some(post) = published.store.find(collection, key).await? else {
                return Ok(None);
            };
            Ok(with_bodies(published, vec![post]).await?.pop())
        })
    }

    fn site(&self) -> BoxFuture<'_, Result<SiteResources, ContentStoreError>> {
        let published = self.current();
        Box::pin(async move { published.store.site().await })
    }
}

/// One store of published posts, without bodies, and where each body is.
#[derive(Debug, Default)]
pub(crate) struct Published {
    store: MemoryContentStore,
    /// Each post's file and language, by the post's identity.
    files: BTreeMap<String, (String, Locale)>,
    folder: PathBuf,
    slugs: SlugMode,
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
        let mut entries = Vec::new();
        let mut files = BTreeMap::new();
        for locale in config.languages() {
            let mut rows: Vec<(&IndexedPost, NaiveDate)> = catalog
                .index
                .posts
                .iter()
                .filter(|row| &row.locale == locale)
                .filter_map(|row| Some((row, row.date?)))
                .collect();
            // Newest first, and posts of one day in slug order, as the
            // index lists them.
            rows.sort_by(|(a, a_date), (b, b_date)| {
                b_date
                    .cmp(a_date)
                    .then_with(|| a.slug.as_str().cmp(b.slug.as_str()))
            });
            for (row, date) in rows {
                let mut post = post_of(config, row, date)?;
                if catalog.featured.contains(&row.path) {
                    post = post.featured();
                }
                let file = (row.path.clone(), locale.clone());
                files.entry(post.id().to_owned()).or_insert(file);
                entries.push((Collection::Posts, post));
            }
        }
        Ok(Self {
            store: MemoryContentStore::new(entries).with_site(catalog.lists.clone()),
            files,
            folder: folder.to_path_buf(),
            slugs: config.slugs(),
        })
    }

    /// The body of the post with identity `id` in every format, read from
    /// its file and rendered now.
    fn bodies(&self, id: &str) -> Result<Bodies, ContentStoreError> {
        let Some((path, locale)) = self.files.get(id) else {
            return Ok(Bodies::default());
        };
        let file = SitePath::within(&self.folder, path).map_err(ContentStoreError::new)?;
        let text = std::fs::read_to_string(file.as_path()).map_err(ContentStoreError::new)?;
        let body = parse_post(file.relative(), &text)
            .map_err(ContentStoreError::new)?
            .body;
        let rendered = render(&body, locale, self.slugs).map_err(ContentStoreError::new)?;
        Ok(Bodies::new(
            &rendered.html,
            &body,
            &plaintext(&rendered.html),
        ))
    }
}

/// Writes the index of `site` in `db` again from `content`, read from the
/// files in `folder`, and returns the published posts and lists it holds,
/// without their bodies.
///
/// # Errors
///
/// Returns what, why and fix when a post cannot be indexed, the database
/// cannot be written, or an address cannot be made from the settings.
pub(crate) async fn published(
    (db, site): (&Database, SiteId),
    folder: &Path,
    config: &Config,
    content: &SiteContent,
) -> Result<Published, UserMessage> {
    let catalog = Catalog::of(folder, config, content)?;
    db.rebuild(site, &catalog.index)
        .await
        .map_err(|error| db_message::of(folder, &error))?;
    Published::of(folder, config, &catalog)
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

/// The index `row` of a post published on `date`, in API shape, without
/// its body.
fn post_of(config: &Config, row: &IndexedPost, date: NaiveDate) -> Result<Post, UserMessage> {
    let locale = &row.locale;
    let path = build_pages::path_of(config, locale, &format!("{}/", row.slug));
    let address = format!("{}{path}", config.address().as_str().trim_end_matches('/'));
    let url = AbsoluteUrl::parse(&address).map_err(|_| build_message::address(&address))?;
    let at = DateTime::<Utc>::from_naive_utc_and_offset(date.and_time(NaiveTime::MIN), Utc);
    let id = format!("{}.{}", locale.as_str(), row.slug);
    let title = row.title.clone().unwrap_or_else(|| row.slug.to_string());
    let mut post = Post::new(&id, row.slug.clone(), &title, url, locale.clone(), at)
        .with_authors(row.author.iter().map(ToString::to_string).collect());
    if let Some(excerpt) = &row.description {
        post = post.with_excerpt(excerpt);
    }
    Ok(post)
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
    async fn a_body_whose_file_is_gone_is_an_error_not_an_empty_post() {
        let (_, config, catalog) = demo();
        let elsewhere = tempfile::tempdir().unwrap();
        let live = LiveContent::default();
        live.replace(Published::of(elsewhere.path(), &config, &catalog).unwrap());
        assert!(live.list(Collection::Posts, &english()).await.is_err());
        let german = PostKey::Id("de.warum-interne-links-wichtig-sind".to_owned());
        assert!(live.find(Collection::Posts, &german).await.is_err());
    }

    #[test]
    fn plaintext_drops_tags_and_reads_entities() {
        assert_eq!(
            plaintext("<p>Links &amp; <a href=\"/x/\">anchors</a></p>\n<p>&lt;3</p>"),
            "Links & anchors <3"
        );
    }
}
