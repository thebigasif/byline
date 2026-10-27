//! Where every old address of a Ghost site leads in Khalid: live posts,
//! pages and author archives, each once (ADR "an import maps every old
//! address"). Ghost's default routes are used: `/<slug>/` for posts and
//! pages and `/author/<slug>/` for authors, under the old site's own path.

use khalid_core::{AbsoluteUrl, Config};

use crate::ghost::site_files::Credited;
use crate::ghost::{Export, ImportedPost};
use crate::redirects::{MapTarget, Matching, moved, path_of, redirects_csv};

/// The redirect map's file, from the site folder, for the host or the
/// redirect manager.
pub const REDIRECTS_CSV: &str = "import/ghost-redirects.csv";

/// One old address and where it leads.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MapEntry {
    /// The old path from the site root, such as `/rye/`.
    pub from: String,
    /// Where it leads.
    pub to: MapTarget,
    /// What the old address was.
    pub source: MapSource,
}

/// What an old address was.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MapSource {
    /// A post or page, by its id in the export.
    Post(String),
    /// An author's archive, by their slug in Khalid.
    Author(String),
}

/// The whole map: live posts and pages in export order, then the archives
/// of the authors who got files. An address already listed is not listed
/// again.
pub(crate) fn redirect_map(
    export: &Export,
    posts: &[ImportedPost],
    authors: &[Credited<'_>],
    old_site: &AbsoluteUrl,
    site: &Config,
) -> Vec<MapEntry> {
    let origin = site.address().as_str().trim_end_matches('/');
    let same_host = old_site.host() == site.address().host();
    let prefix = path_of(old_site);
    let prefix = format!("{}/", prefix.trim_end_matches('/'));
    let mut map: Vec<MapEntry> = Vec::new();
    let mut add = |from: String, new: &str, source: MapSource| {
        if map.iter().any(|entry| entry.from == from) {
            return;
        }
        let to = if same_host && from == new {
            MapTarget::Kept
        } else {
            moved(&format!("{origin}{new}"))
        };
        map.push(MapEntry { from, to, source });
    };
    for post in posts {
        let (Some(new), Some(ghost)) = (
            post.address.as_deref(),
            export.posts.iter().find(|ghost| ghost.id == post.id),
        ) else {
            continue;
        };
        if !ghost.slug.is_empty() {
            let from = format!("{prefix}{}/", ghost.slug);
            add(from, new, MapSource::Post(post.id.clone()));
        }
    }
    for Credited { author, slug, .. } in authors {
        let new = format!("{}authors/{slug}/", site.mount());
        let from = format!("{prefix}author/{}/", author.slug);
        add(from, &new, MapSource::Author(slug.clone()));
    }
    map
}

/// The map as CSV, every entry that redirects.
pub(crate) fn ghost_csv(map: &[MapEntry]) -> String {
    redirects_csv(
        map.iter()
            .map(|entry| (entry.from.as_str(), &entry.to, Matching::Exact)),
    )
}
