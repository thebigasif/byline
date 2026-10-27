//! The automatic 301 when a page's address changes (KH-SEO-8): each
//! render's addresses are compared with the last ones, and every page
//! that moved gets a stored permanent move from its old address, made by
//! `slug_change`.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use khalid_api::redirect_store::{Origin, RedirectStore, RedirectStoreError, source_of};
use khalid_core::Config;
use khalid_db::{ContentIndex, Database, SiteId};
use khalid_seo::{RedirectEntry, RedirectRule, RedirectTable, UrlPath, moved_addresses};

use crate::live_redirects::LiveRedirects;
use crate::site_addresses;

/// The address of each page the index in `db` held before this run, by
/// file, read for each language and collection `index` has now. A page
/// whose address changed while the server was stopped is caught by
/// comparing this with the first render.
pub(crate) async fn indexed(
    (db, site): (&Database, SiteId),
    config: &Config,
    index: &ContentIndex,
) -> BTreeMap<String, UrlPath> {
    let mut groups: Vec<_> = index
        .posts
        .iter()
        .map(|p| (&p.locale, &p.collection))
        .collect();
    groups.sort();
    groups.dedup();
    let mut before = ContentIndex::default();
    for (locale, collection) in groups {
        match db.posts(site, locale, collection, u32::MAX).await {
            Ok(posts) => before.posts.extend(posts),
            Err(error) => tracing::warn!(%error, "the last addresses could not be read"),
        }
    }
    site_addresses::of(config, &before)
}

/// Stores a permanent move for every page whose address is not the same
/// in `after` as in `before`, made at `at`.
///
/// A stored rule from a page's new address is removed first, since the
/// page answers there again; a move that would take an address that has a
/// rule, or close a loop, is left out and logged.
pub(crate) async fn store(
    redirects: &LiveRedirects,
    (before, after): (&BTreeMap<String, UrlPath>, &BTreeMap<String, UrlPath>),
    at: DateTime<Utc>,
) {
    for rule in moved_addresses(before, after) {
        if let Err(error) = store_one(redirects, rule, at).await {
            tracing::warn!(%error, "a moved page got no redirect");
        }
    }
}

async fn store_one(
    redirects: &LiveRedirects,
    rule: RedirectRule,
    at: DateTime<Utc>,
) -> Result<(), RedirectStoreError> {
    let to = rule.redirect.location().map(ToString::to_string);
    for stale in redirects.list().await? {
        if Some(source_of(&stale.entry)) == to.as_deref() {
            redirects.remove(stale.id).await?;
        }
    }
    let entry = RedirectEntry::Exact(rule);
    let mut all: Vec<_> = redirects
        .list()
        .await?
        .into_iter()
        .map(|s| s.entry)
        .collect();
    all.push(entry.clone());
    if let Err(error) = RedirectTable::from_entries(all) {
        tracing::debug!(%error, "a moved page's redirect was left out");
        return Ok(());
    }
    match redirects.add(vec![entry], Origin::SlugChange, at).await {
        Err(RedirectStoreError::Taken { from }) => {
            tracing::debug!(%from, "a moved page's old address already has a redirect");
            Ok(())
        }
        added => added.map(|_| ()),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::TimeZone;
    use khalid_api::redirect_store::MemoryRedirectStore;

    use super::*;

    fn at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 3, 9, 0, 0).unwrap()
    }

    fn addresses(pairs: &[(&str, &str)]) -> BTreeMap<String, UrlPath> {
        let path = |at: &str| UrlPath::try_new(at).unwrap();
        pairs
            .iter()
            .map(|(file, at)| ((*file).to_owned(), path(at)))
            .collect()
    }

    async fn rows(live: &LiveRedirects) -> Vec<(String, Option<String>, Origin)> {
        let listed = live.list().await.unwrap();
        let columns = listed.iter().map(|s| (s.entry.columns(), s.origin));
        columns.map(|(c, origin)| (c.from, c.to, origin)).collect()
    }

    #[tokio::test]
    async fn a_moved_page_gets_a_301_and_moving_back_leaves_no_loop() {
        let live = LiveRedirects::new(Arc::new(MemoryRedirectStore::default()));
        let first = addresses(&[("en/links.md", "/blog/internal-links/")]);
        let renamed = addresses(&[("en/links.md", "/blog/why-internal-links-matter/")]);
        store(&live, (&first, &renamed), at()).await;
        let old = "/blog/internal-links/".to_owned();
        let new = "/blog/why-internal-links-matter/".to_owned();
        let moved = (old.clone(), Some(new.clone()), Origin::SlugChange);
        assert_eq!(rows(&live).await, [moved]);
        let path = UrlPath::try_new(&old).unwrap();
        assert_eq!(
            live.table().resolve(&path).map(|hit| hit.status()),
            Some(301)
        );

        store(&live, (&renamed, &first), at()).await;
        assert_eq!(rows(&live).await, [(new, Some(old), Origin::SlugChange)]);
    }

    #[tokio::test]
    async fn an_old_address_with_a_rule_keeps_it() {
        let live = LiveRedirects::new(Arc::new(MemoryRedirectStore::default()));
        let made =
            khalid_seo::read_redirects_csv("from,to,status\n/blog/a/,/blog/elsewhere/,302\n");
        live.add(made.unwrap(), Origin::Manual, at()).await.unwrap();
        let before = addresses(&[("en/a.md", "/blog/a/")]);
        let after = addresses(&[("en/a.md", "/blog/b/")]);
        store(&live, (&before, &after), at()).await;
        let kept = (
            "/blog/a/".to_owned(),
            Some("/blog/elsewhere/".to_owned()),
            Origin::Manual,
        );
        assert_eq!(rows(&live).await, [kept]);
    }
}
