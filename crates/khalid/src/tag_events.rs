//! The `tag.added` webhook `khalid serve` fires when a live post first
//! names a tag the site had on no post before (audit brief G25). A tag is
//! made by naming it in a post's front matter, so the site's tags are the
//! tags of its live posts.

use std::collections::BTreeMap;

use khalid_api::resources::Term;
use khalid_content::SiteContent;
use khalid_core::Config;
use khalid_webhooks::Event;
use serde::Serialize;

use crate::announce::SiteAnnounce;
use crate::listed_post::ListedTag;

/// Every tag the live posts of one render name, by `<language>.<slug>`.
#[derive(Debug, Clone, Default)]
pub(crate) struct Tags(BTreeMap<String, Term>);

/// The resource a `tag.added` delivery carries.
#[derive(Debug, Serialize)]
struct Added {
    tag: Term,
}

impl Tags {
    /// The tags of the posts in `content`, each slug made from its name as
    /// the tag's page is, with the site's slug rules in `config`. The first
    /// post in path order that names a tag gives its name.
    pub(crate) fn of(config: &Config, content: &SiteContent) -> Self {
        let mut tags = BTreeMap::new();
        for doc in &content.posts {
            for tag in ListedTag::all(&doc.front.tags, &doc.locale, config.slugs()) {
                let id = format!("{}.{}", doc.locale.as_str(), tag.slug.as_str());
                let term = || Term::new(tag.slug, &tag.name, doc.locale.clone());
                tags.entry(id).or_insert_with(term);
            }
        }
        Self(tags)
    }

    /// Fires `tag.added` on `announce` for each tag in `self` that was not
    /// in `before`. A tag no post names any more fires nothing.
    pub(crate) fn fire_since(&self, before: &Self, announce: &SiteAnnounce) {
        for tag in self.added_since(before) {
            announce.fire(Event::TagAdded, Added { tag: tag.clone() });
        }
    }

    /// The tags in `self` that were not in `before`.
    fn added_since<'a>(&'a self, before: &Self) -> Vec<&'a Term> {
        let added = self.0.iter().filter(|(id, _)| !before.0.contains_key(*id));
        added.map(|(_, tag)| tag).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use khalid_content::{ContentTree, read_content};
    use khalid_core::Environment;

    use super::*;

    /// The demo site's tags, after its first `tagged` posts each name
    /// `crawl budget` too. The demo posts name no tags of their own, and
    /// the German ones come first.
    fn demo(tagged: usize) -> Tags {
        let mut folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        folder.push("../../fixtures/demo-site");
        let config = Config::load(&folder, &Environment::empty()).unwrap();
        let mut content = read_content(&ContentTree::walk(&folder).unwrap()).unwrap();
        for doc in content.posts.iter_mut().take(tagged) {
            doc.front.tags.push("crawl budget".to_owned());
        }
        Tags::of(&config, &content)
    }

    #[test]
    fn a_tag_named_by_several_posts_is_one_tag_named_by_language_and_slug() {
        let tags = demo(2);
        let ids: Vec<&str> = tags.0.keys().map(String::as_str).collect();
        assert_eq!(ids, ["de.crawl-budget"]);
        let json = serde_json::to_value(&tags.0["de.crawl-budget"]).unwrap();
        assert_eq!(json["name"], "crawl budget", "{json}");
    }

    #[test]
    fn only_a_tag_new_to_the_site_is_added() {
        let (none, tagged) = (demo(0), demo(1));
        let added = serde_json::to_value(tagged.added_since(&none)).unwrap();
        assert_eq!(added[0]["id"], "de.crawl-budget", "{added}");
        assert_eq!(added.as_array().map(Vec::len), Some(1), "{added}");
        assert!(tagged.added_since(&demo(2)).is_empty(), "known already");
        assert!(
            none.added_since(&tagged).is_empty(),
            "a tag gone adds nothing"
        );
    }
}
