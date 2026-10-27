//! Where every old address of a `WordPress` site leads in Khalid: posts,
//! pages, images' pages, category archives and the Redirection plugin's
//! rules, each once.

use khalid_core::{AbsoluteUrl, Config};
use khalid_routes::Routes;

use crate::redirects::{MapTarget, Matching, moved, path_of, redirects_csv};
use crate::wordpress::names::category_slug;
use crate::wordpress::{
    Export, ImportedItem, ItemKind, Redirection, RedirectionRule, RuleSource, RuleTarget,
};

/// One old address and where it leads.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MapEntry {
    /// The old path from the site root, such as `/2024/03/rye/`, or a
    /// regular expression over paths.
    pub from: String,
    /// Whether `from` is one path or a pattern.
    pub matching: Matching,
    /// Where it leads.
    pub to: MapTarget,
    /// What the old address was.
    pub source: MapSource,
}

/// What an old address was.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MapSource {
    /// A post, page or image page, by its id in the export.
    Item(u64),
    /// A category's archive, by its slug in `WordPress`.
    Category(String),
    /// A rule of the Redirection plugin, by its id there.
    Rule(u64),
}

/// An item that was live on the old site and is live in Khalid.
struct Live<'a> {
    id: u64,
    /// Its old path.
    old: &'a str,
    /// Its full new address.
    new: String,
    /// Whether the address is the same, host and path.
    same: bool,
}

/// The whole map: the plugin's rules first, because they answered before
/// `WordPress` did, then posts and pages in export order, images' pages and
/// category archives. An address already listed is not listed again.
pub(crate) fn redirect_map(
    export: &Export,
    redirection: Option<&Redirection>,
    items: &[ImportedItem],
    site: &Config,
    routes: &Routes,
) -> Vec<MapEntry> {
    let origin = site.address().as_str().trim_end_matches('/');
    let same_host = export.site.address.host() == site.address().host();
    let live: Vec<Live<'_>> = items
        .iter()
        .filter_map(|item| {
            let old = path_of(export.item(item.id)?.link.as_ref()?);
            let new = item.address.as_deref()?;
            let same = same_host && old == new;
            Some(Live {
                id: item.id,
                old,
                new: format!("{origin}{new}"),
                same,
            })
        })
        .collect();
    let rules = redirection.map_or(&[][..], |redirection| redirection.rules.as_slice());
    let mut map: Vec<MapEntry> = rules.iter().map(|rule| rule_entry(rule, &live)).collect();
    for item in &live {
        let to = if item.same {
            MapTarget::Kept
        } else {
            moved(&item.new)
        };
        add(&mut map, item.old, to, MapSource::Item(item.id));
    }
    for image in export
        .items
        .iter()
        .filter(|item| item.kind == ItemKind::Attachment)
    {
        let (Some(link), Some(parent)) = (&image.link, image.parent) else {
            continue;
        };
        if let Some(post) = live.iter().find(|item| item.id == parent) {
            add(
                &mut map,
                path_of(link),
                moved(&post.new),
                MapSource::Item(image.id),
            );
        }
    }
    for term in &export.categories {
        let Some(slug) = category_slug(&term.slug, export, site) else {
            continue;
        };
        let new = format!(
            "{origin}{}{}/{slug}/",
            site.mount(),
            routes.category_prefix()
        );
        let old = category_path(term.slug.as_str(), export);
        add(
            &mut map,
            &old,
            moved(&new),
            MapSource::Category(term.slug.clone()),
        );
    }
    map
}

/// A Redirection rule as an entry, its target sent straight to the new
/// address when it pointed at a post that moved, so no chain is left.
fn rule_entry(rule: &RedirectionRule, live: &[Live<'_>]) -> MapEntry {
    let (from, matching) = match &rule.from {
        RuleSource::Path(path) => (path.clone(), Matching::Exact),
        RuleSource::Pattern(pattern) => (pattern.clone(), Matching::Pattern),
    };
    let to = match &rule.to {
        RuleTarget::Moved { to, status } => {
            let old =
                AbsoluteUrl::parse(to).map_or_else(|_| to.clone(), |url| path_of(&url).to_owned());
            let post = live.iter().find(|item| item.old == old);
            let to = post.map_or_else(|| to.clone(), |post| post.new.clone());
            MapTarget::Moved {
                to,
                status: *status,
            }
        }
        RuleTarget::Gone => MapTarget::Gone,
        RuleTarget::Unavailable => MapTarget::Unavailable,
    };
    MapEntry {
        from,
        matching,
        to,
        source: MapSource::Rule(rule.id),
    }
}

/// Adds an exact entry unless the address is listed already.
fn add(map: &mut Vec<MapEntry>, from: &str, to: MapTarget, source: MapSource) {
    if !map.iter().any(|entry| entry.from == from) {
        let from = from.to_owned();
        map.push(MapEntry {
            from,
            matching: Matching::Exact,
            to,
            source,
        });
    }
}

/// A category's archive in `WordPress`: `/category/` and the slugs of its
/// parents and its own, such as `/category/baking/sourdough/`.
fn category_path(slug: &str, export: &Export) -> String {
    let mut chain = vec![slug];
    let mut current = slug;
    while let Some(parent) = export
        .categories
        .iter()
        .find(|term| term.slug == current)
        .and_then(|term| term.parent.as_deref())
        .filter(|parent| !chain.contains(parent))
    {
        chain.push(parent);
        current = parent;
    }
    chain.reverse();
    format!("/category/{}/", chain.join("/"))
}

/// The map as CSV, every entry that redirects.
pub(crate) fn wordpress_csv(map: &[MapEntry]) -> String {
    redirects_csv(
        map.iter()
            .map(|entry| (entry.from.as_str(), &entry.to, entry.matching)),
    )
}
