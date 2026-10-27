//! What the Content API answers with under `khalid mcp`: the published
//! posts read again from the files on each request, so a post the agent
//! has just published is listed at once. `khalid serve` keeps its store
//! up to date by watching the folder instead.

use std::io;
use std::path::{Path, PathBuf};

use khalid_api::content_store::{
    BoxFuture, ContentStore, ContentStoreError, Listed, PostKey, SiteResources,
};
use khalid_api::query::ListQuery;
use khalid_api::resources::{Collection, Post};
use khalid_content::{ContentTree, read_content};
use khalid_core::Config;

use crate::site_content::{Catalog, LiveContent, Published};

/// The published posts of the site in one folder, read on each request.
#[derive(Debug, Clone)]
pub(crate) struct FreshContent {
    folder: PathBuf,
    config: Config,
    live: LiveContent,
}

impl FreshContent {
    /// The site in `folder`, with its checked `config`.
    pub(crate) fn new(folder: &Path, config: Config) -> Self {
        Self {
            folder: folder.to_path_buf(),
            config,
            live: LiveContent::default(),
        }
    }

    /// Reads the files on a blocking thread (RULES.md BL-RUN-7) and serves
    /// what they hold now.
    async fn now(&self) -> Result<&LiveContent, ContentStoreError> {
        let (folder, config) = (self.folder.clone(), self.config.clone());
        let published = tokio::task::spawn_blocking(move || {
            let content = ContentTree::walk(&folder)
                .and_then(|tree| read_content(&tree))
                .map_err(ContentStoreError::new)?;
            Catalog::of(&folder, &config, &content)
                .and_then(|catalog| Published::of(&folder, &config, &catalog))
                .map_err(|message| ContentStoreError::new(io::Error::other(message.to_string())))
        });
        let published = published.await.map_err(ContentStoreError::new)??;
        self.live.replace(published);
        Ok(&self.live)
    }
}

impl ContentStore for FreshContent {
    fn list<'a>(
        &'a self,
        collection: Collection,
        query: &'a ListQuery,
    ) -> BoxFuture<'a, Result<Listed<Post>, ContentStoreError>> {
        Box::pin(async move { self.now().await?.list(collection, query).await })
    }

    fn find<'a>(
        &'a self,
        collection: Collection,
        key: &'a PostKey,
    ) -> BoxFuture<'a, Result<Option<Post>, ContentStoreError>> {
        Box::pin(async move { self.now().await?.find(collection, key).await })
    }

    fn site(&self) -> BoxFuture<'_, Result<SiteResources, ContentStoreError>> {
        Box::pin(async move { self.now().await?.site().await })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use khalid_api::query::RawListQuery;
    use khalid_core::Environment;

    use super::*;

    fn copy_dir(from: &Path, into: &Path) {
        fs::create_dir_all(into).unwrap();
        for entry in fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let mut target = into.to_path_buf();
            target.push(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), &target).unwrap();
            }
        }
    }

    async fn slugs(content: &FreshContent) -> Vec<String> {
        let query = ListQuery::try_from(&RawListQuery::default()).unwrap();
        let listed = content.list(Collection::Posts, &query).await.unwrap();
        listed
            .items()
            .iter()
            .map(|post| post.slug().to_string())
            .collect()
    }

    #[tokio::test]
    async fn a_post_saved_after_start_is_listed_on_the_next_request() {
        let mut fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        fixture.push("../../fixtures/demo-site");
        let site = tempfile::tempdir().unwrap();
        copy_dir(&fixture, site.path());
        let config = Config::load(site.path(), &Environment::empty()).unwrap();
        let content = FreshContent::new(site.path(), config);
        let before = slugs(&content).await;
        assert!(
            before.contains(&"how-to-write-a-title-tag".to_owned()),
            "{before:?}"
        );
        assert!(
            !before.contains(&"choosing-anchor-text".to_owned()),
            "{before:?}"
        );

        let mut saved = site.path().to_path_buf();
        saved.push("content/en/posts/2026-10-20-choosing-anchor-text.md");
        let post = "---\ntitle: Choosing anchor text\nslug: choosing-anchor-text\n\
            date: 2026-10-20\nauthor: asif\n---\n\nAnchor text is the words of a link.\n";
        fs::write(&saved, post).unwrap();
        let after = slugs(&content).await;
        assert!(
            after.contains(&"choosing-anchor-text".to_owned()),
            "{after:?}"
        );
        assert_eq!(after.len(), before.len() + 1, "{after:?}");
    }
}
