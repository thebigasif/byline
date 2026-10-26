//! Wikilinks: `[[slug]]` and `[[slug|text]]` in markdown, turned into links
//! to the pages they name at build (plan section 5.11).

use std::collections::BTreeMap;

use comrak::nodes::{AstNode, NodeLink, NodeValue};
use khalid_core::Slug;

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
