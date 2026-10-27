//! The address of each page in the content index, by the file it comes
//! from: what "redirect this" picks the nearest page from, and what the
//! automatic 301 compares when an address changes (KH-SEO-8).

use std::collections::BTreeMap;
use std::sync::Arc;

use khalid_api::content_store::BoxFuture;
use khalid_api::redirect_desk::{PageAddresses, PageAddressesError};
use khalid_core::Config;
use khalid_db::ContentIndex;
use khalid_seo::UrlPath;
use tokio::sync::watch;

use crate::build_pages;

/// Each dated entry's address from the site root, by its file's path,
/// the address `build_pages` gives its page. An entry whose address is not
/// a clean path has no page, so it is left out.
pub(crate) fn of(config: &Config, index: &ContentIndex) -> BTreeMap<String, UrlPath> {
    index
        .posts
        .iter()
        .filter(|post| post.date.is_some())
        .filter_map(|post| {
            let path = build_pages::path_of(config, &post.locale, &format!("{}/", post.slug));
            Some((post.path.clone(), UrlPath::try_new(&path).ok()?))
        })
        .collect()
}

/// The addresses served now, replaced after each render. Cloning shares
/// them.
#[derive(Debug, Clone)]
pub(crate) struct LiveAddresses(Arc<watch::Sender<Arc<[UrlPath]>>>);

impl Default for LiveAddresses {
    fn default() -> Self {
        Self(Arc::new(watch::channel(Arc::default()).0))
    }
}

impl LiveAddresses {
    /// Serves the addresses in `by_file`, in the order of their files.
    pub(crate) fn replace(&self, by_file: &BTreeMap<String, UrlPath>) {
        let addresses: Arc<[UrlPath]> = by_file.values().cloned().collect();
        self.0.send_replace(addresses);
    }
}

impl PageAddresses for LiveAddresses {
    fn addresses(&self) -> BoxFuture<'_, Result<Vec<UrlPath>, PageAddressesError>> {
        let addresses = self.0.borrow().to_vec();
        Box::pin(std::future::ready(Ok(addresses)))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use khalid_content::{ContentTree, read_content};
    use khalid_core::Environment;

    use super::*;

    #[tokio::test]
    async fn each_dated_demo_post_has_its_address_by_file() {
        let mut folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        folder.push("../../fixtures/demo-site");
        let config = Config::load(&folder, &Environment::empty()).unwrap();
        let content = read_content(&ContentTree::walk(&folder).unwrap()).unwrap();
        let index = crate::index::of_content(&folder, &content).unwrap();
        let by_file = of(&config, &index);
        let english = "content/en/posts/2026-09-14-why-internal-links-matter.md";
        let at = by_file.get(english).map(UrlPath::as_str);
        assert_eq!(at, Some("/blog/why-internal-links-matter/"));
        assert!(
            by_file
                .values()
                .any(|a| a.as_str().starts_with("/blog/de/"))
        );
        let dated = index.posts.iter().filter(|p| p.date.is_some()).count();
        assert_eq!(by_file.len(), dated);

        let live = LiveAddresses::default();
        assert_eq!(live.addresses().await.unwrap(), []);
        live.replace(&by_file);
        assert_eq!(live.addresses().await.unwrap().len(), dated);
    }
}
