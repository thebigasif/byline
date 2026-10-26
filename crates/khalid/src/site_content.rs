//! What the Content API answers with on a running site: the published posts
//! of the content index in `data/site.db`, with their bodies, and the site's
//! lists, held in memory and replaced as a whole after each change, like the
//! pages.

use std::path::Path;
use std::sync::{Arc, PoisonError, RwLock};

use chrono::{DateTime, NaiveTime, Utc};
use khalid_api::content_store::{
    BoxFuture, ContentStore, ContentStoreError, Listed, MemoryContentStore, PostKey, SiteResources,
};
use khalid_api::query::ListQuery;
use khalid_api::resources::{Bodies, Collection, Post};
use khalid_content::{SiteContent, render};
use khalid_core::{AbsoluteUrl, Config, UserMessage};
use khalid_db::{Database, SiteId};

use crate::{build_message, build_pages, db_message, index, site_lists};

/// The collection the index keeps posts under.
const POSTS: &str = "posts";

/// The published content readers see now. Cloning shares it, and a
/// refresh swaps it whole, so a request never sees half a rebuild.
#[derive(Debug, Clone, Default)]
pub(crate) struct LiveContent(Arc<RwLock<Arc<MemoryContentStore>>>);

impl LiveContent {
    /// Serves `store` until it is replaced.
    pub(crate) fn replace(&self, store: MemoryContentStore) {
        let mut slot = self.0.write().unwrap_or_else(PoisonError::into_inner);
        *slot = Arc::new(store);
    }

    fn current(&self) -> Arc<MemoryContentStore> {
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
        let current = self.current();
        Box::pin(async move { current.list(collection, query).await })
    }

    fn find<'a>(
        &'a self,
        collection: Collection,
        key: &'a PostKey,
    ) -> BoxFuture<'a, Result<Option<Post>, ContentStoreError>> {
        let current = self.current();
        Box::pin(async move { current.find(collection, key).await })
    }

    fn site(&self) -> BoxFuture<'_, Result<SiteResources, ContentStoreError>> {
        let current = self.current();
        Box::pin(async move { current.site().await })
    }
}

/// Writes the index of `site` in `db` again from the files in `folder`,
/// then reads its published posts back, in every language, with the bodies
/// from `content`, and the site's lists. A post is published when it has a
/// date, as on the public site.
///
/// # Errors
///
/// Returns what, why and fix when a file cannot be indexed, the database
/// cannot be written or read, or a body cannot be rendered.
pub(crate) async fn published(
    db: &Database,
    site: SiteId,
    folder: &Path,
    config: &Config,
    content: &SiteContent,
) -> Result<MemoryContentStore, UserMessage> {
    let unusable = |error| db_message::of(folder, &error);
    let indexed = index::of(folder)?;
    db.rebuild(site, &indexed).await.map_err(unusable)?;
    let mut entries = Vec::new();
    for locale in config.languages() {
        let rows = db
            .posts(site, locale, POSTS, u32::MAX)
            .await
            .map_err(unusable)?;
        for row in rows {
            let Some(date) = row.date else {
                continue;
            };
            let document = content
                .posts
                .iter()
                .find(|doc| index::slashed(&doc.path).as_deref() == Some(row.path.as_str()));
            let Some(document) = document else {
                continue;
            };
            let rendered = render(&document.body, locale, config.slugs())
                .map_err(|error| build_message::block(&document.path, &error))?;
            let path = build_pages::path_of(config, locale, &format!("{}/", row.slug));
            let address = format!("{}{path}", config.address().as_str().trim_end_matches('/'));
            let url = AbsoluteUrl::parse(&address).map_err(|_| build_message::address(&address))?;
            let at = DateTime::<Utc>::from_naive_utc_and_offset(date.and_time(NaiveTime::MIN), Utc);
            let id = format!("{}.{}", locale.as_str(), row.slug);
            let title = row.title.clone().unwrap_or_else(|| row.slug.to_string());
            let mut post = Post::new(&id, row.slug.clone(), &title, url, locale.clone(), at)
                .with_authors(row.author.iter().map(ToString::to_string).collect())
                .with_bodies(Bodies::new(
                    &rendered.html,
                    &document.body,
                    &plaintext(&rendered.html),
                ));
            if let Some(excerpt) = &row.description {
                post = post.with_excerpt(excerpt);
            }
            if document.front.featured {
                post = post.featured();
            }
            entries.push((Collection::Posts, post));
        }
    }
    let lists = site_lists::of(db, site, folder, config, content).await?;
    Ok(MemoryContentStore::new(entries).with_site(lists))
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
    use std::path::PathBuf;

    use khalid_api::query::RawListQuery;
    use khalid_content::{ContentTree, read_content};
    use khalid_core::{Environment, Slug};

    use super::*;

    fn demo() -> PathBuf {
        let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        demo.push("../../fixtures/demo-site");
        demo
    }

    #[tokio::test]
    async fn the_demo_posts_come_back_from_the_index_with_their_bodies() {
        let folder = demo();
        let config = Config::load(&folder, &Environment::empty()).unwrap();
        let content = read_content(&ContentTree::walk(&folder).unwrap()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let mut file = dir.path().to_path_buf();
        file.push("site.db");
        let db = Database::open(&file).await.unwrap();
        db.migrate().await.unwrap();
        let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
        let store = published(&db, site, &folder, &config, &content)
            .await
            .unwrap();
        let live = LiveContent::default();
        live.replace(store);

        let mut raw = RawListQuery::default();
        raw.filter = Some("language:en".to_owned());
        let query = ListQuery::try_from(&raw).unwrap();
        let listed = live.list(Collection::Posts, &query).await.unwrap();
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
        db.close().await;
    }

    #[test]
    fn plaintext_drops_tags_and_reads_entities() {
        assert_eq!(
            plaintext("<p>Links &amp; <a href=\"/x/\">anchors</a></p>\n<p>&lt;3</p>"),
            "Links & anchors <3"
        );
    }
}
