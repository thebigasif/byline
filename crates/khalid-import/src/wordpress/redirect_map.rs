//! Where every old address of a `WordPress` site leads in Khalid: posts,
//! pages, images' pages and category archives, each once.

use khalid_core::Config;
use khalid_routes::Routes;

use crate::wordpress::names::{category_slug, path_of};
use crate::wordpress::{Export, ImportedItem, ItemKind, MoveStatus};

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

/// How an entry matches old addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Matching {
    /// One path, exactly.
    Exact,
    /// A regular expression over the path, from the Redirection plugin.
    Pattern,
}

/// Where an old address leads.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MapTarget {
    /// Nowhere new: the address is the same in Khalid.
    Kept,
    /// To a new address, a full one or a path the plugin wrote.
    Moved {
        /// Where the visitor goes.
        to: String,
        /// How long the move lasts.
        status: MoveStatus,
    },
    /// 410: gone for good.
    Gone,
    /// 451: withheld for legal reasons.
    Unavailable,
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

/// The whole map: posts and pages in export order, then images' pages and
/// category archives. An address already listed is not listed again.
pub(crate) fn redirect_map(
    export: &Export,
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
    let mut map = Vec::new();
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

/// A permanent move to `to`.
fn moved(to: &str) -> MapTarget {
    MapTarget::Moved {
        to: to.to_owned(),
        status: MoveStatus::Permanent,
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
