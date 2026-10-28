//! Where every old address of a file from a markdown folder leads in
//! Khalid, each once (ADR "an import maps every old address"): the
//! address its generator gave it and its Hugo `aliases`.

use khalid_core::{AbsoluteUrl, Config};

use crate::folder::ImportedPost;
use crate::redirects::{MapTarget, Matching, moved, path_of, redirects_csv};

/// The redirect map's file, from the site folder, for the host or the
/// redirect manager.
pub const REDIRECTS_CSV: &str = "import/markdown-redirects.csv";

/// One old address and where it leads.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MapEntry {
    /// The whole old address.
    pub from: AbsoluteUrl,
    /// Where it leads.
    pub to: MapTarget,
    /// The file's path inside the folder.
    pub source: String,
}

/// The whole map, in path order. An address already listed is not listed
/// again. Drafts were never public, so they have none.
pub(crate) fn redirect_map(posts: &[ImportedPost], site: &Config) -> Vec<MapEntry> {
    let origin = site.address().as_str().trim_end_matches('/');
    let mut map: Vec<MapEntry> = Vec::new();
    for post in posts {
        let Some(new) = post.address.as_deref() else {
            continue;
        };
        for from in &post.old {
            if map.iter().any(|entry| entry.from == *from) {
                continue;
            }
            let same = from.host() == site.address().host() && path_of(from) == new;
            let to = if same {
                MapTarget::Kept
            } else {
                moved(&format!("{origin}{new}"))
            };
            map.push(MapEntry {
                from: from.clone(),
                to,
                source: post.source.clone(),
            });
        }
    }
    map
}

/// The map as CSV, every entry that redirects. An address on the new
/// site's own host is written as its path; any other is written whole.
pub(crate) fn folder_csv(map: &[MapEntry], site: &Config) -> String {
    redirects_csv(map.iter().map(|entry| {
        let from = if entry.from.host() == site.address().host() {
            path_of(&entry.from)
        } else {
            entry.from.as_str()
        };
        (from, &entry.to, Matching::Exact)
    }))
}
