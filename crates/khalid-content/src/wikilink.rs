//! Wikilinks: `[[slug]]` and `[[slug|text]]` in markdown, turned into links
//! to the pages they name at build (plan section 5.11).

use std::collections::BTreeMap;

use comrak::nodes::{AstNode, NodeLink, NodeValue};
use khalid_core::{Config, Locale, Slug};
use khalid_routes::{PostAddress, Routes};

use crate::SiteContent;

/// The pages a wikilink can reach in one language, by slug.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Wikilinks {
    targets: BTreeMap<String, WikiTarget>,
}

impl Wikilinks {
    /// No targets, so every wikilink shows as plain text.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Every post, page and glossary term written in `locale`, at the address
    /// `routes` gives it on `site`, so a wikilink reaches pages in its own
    /// language only. When two share a slug, a post wins over a page and a
    /// page over a glossary term. Pages sit at `/<slug>/` under the mount and
    /// language prefix, where `khalid build` writes them. An undated post in a
    /// collection whose addresses carry the date has no address yet, so no
    /// wikilink reaches it.
    #[must_use]
    pub fn for_language(
        content: &SiteContent,
        site: &Config,
        routes: &Routes,
        locale: &Locale,
    ) -> Self {
        let mut links = Self::new();
        let named = |slug: Option<&Slug>, title: Option<&str>| {
            let slug = match (slug, title) {
                (Some(slug), _) => slug.clone(),
                (None, Some(title)) => Slug::from_title(title, locale, site.slugs()).ok()?,
                (None, None) => return None,
            };
            let title = title.map_or_else(|| slug.to_string(), str::to_owned);
            Some((slug, title))
        };
        for doc in content.posts.iter().filter(|doc| &doc.locale == locale) {
            let front = &doc.front;
            let Some((slug, title)) = named(front.slug.as_ref(), front.title.as_deref()) else {
                continue;
            };
            let post = PostAddress::new(&doc.collection, locale, &slug, front.date);
            if let Ok(address) = routes.address(site, &post) {
                links.insert(&slug, WikiTarget::new(address, title));
            }
        }
        let language = if locale == site.default_language() {
            String::new()
        } else {
            format!("{locale}/")
        };
        for doc in content.pages.iter().filter(|doc| &doc.locale == locale) {
            let front = &doc.front;
            if let Some((slug, title)) = named(front.slug.as_ref(), front.title.as_deref()) {
                let address = format!("{}{language}{slug}/", site.mount());
                links.insert(&slug, WikiTarget::new(address, title));
            }
        }
        for doc in content.glossary.iter().filter(|doc| &doc.locale == locale) {
            let front = &doc.front;
            let Some((slug, title)) = named(front.slug.as_ref(), front.term.as_deref()) else {
                continue;
            };
            let term = PostAddress::new(&doc.collection, locale, &slug, None);
            if let Ok(address) = routes.address(site, &term) {
                links.insert(&slug, WikiTarget::new(address, title));
            }
        }
        links
    }

    /// Lets `[[slug]]` reach `target`. A slug that is already taken keeps
    /// its first target, so the caller decides which kind of page wins.
    pub fn insert(&mut self, slug: &Slug, target: WikiTarget) {
        self.targets
            .entry(slug.as_str().to_owned())
            .or_insert(target);
    }

    /// The page `[[slug]]` reaches, if any.
    #[must_use]
    pub fn get(&self, slug: &str) -> Option<&WikiTarget> {
        self.targets.get(slug)
    }

    /// How many slugs have a target.
    #[must_use]
    pub fn len(&self) -> usize {
        self.targets.len()
    }

    /// Whether no slug has a target.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }
}

/// The page a wikilink reaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiTarget {
    address: String,
    title: String,
}

impl WikiTarget {
    /// A page at `address`, a path from the site root, titled `title`.
    pub fn new(address: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            title: title.into(),
        }
    }

    /// The page's path from the site root, such as `/blog/glossary/seo/`.
    #[must_use]
    pub fn address(&self) -> &str {
        &self.address
    }

    /// The page's title, the link text when the wikilink names none.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }
}

/// Turns every wikilink under `root` into a link to its target, or into its
/// text alone when `links` has no target for it. A wikilink with no text
/// after a pipe shows its target's title.
pub(crate) fn resolve<'a>(root: &'a AstNode<'a>, links: &Wikilinks) {
    let wikilinks: Vec<_> = root
        .descendants()
        .filter(|node| matches!(node.data.borrow().value, NodeValue::WikiLink(_)))
        .collect();
    for node in wikilinks {
        let slug = match &node.data.borrow().value {
            NodeValue::WikiLink(link) => link.url.clone(),
            _ => continue,
        };
        let Some(target) = links.get(&slug) else {
            let children: Vec<_> = node.children().collect();
            for child in children {
                node.insert_before(child);
            }
            node.detach();
            continue;
        };
        if let Some(only) = node
            .first_child()
            .filter(|child| child.next_sibling().is_none())
        {
            let mut text = only.data.borrow_mut();
            if matches!(&text.value, NodeValue::Text(written) if *written == slug) {
                text.value = NodeValue::Text(target.title.clone().into());
            }
        }
        node.data.borrow_mut().value = NodeValue::Link(Box::new(NodeLink {
            url: target.address.clone(),
            title: String::new(),
        }));
    }
}
