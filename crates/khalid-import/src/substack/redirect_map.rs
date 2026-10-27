//! Where the old address of every published Substack post leads in
//! Khalid, each once (ADR "an import maps every old address"). A Substack
//! post answers at `/p/<slug>` on the publication's address, a
//! `substack.com` subdomain or its own domain.

use khalid_core::{AbsoluteUrl, Config};

use crate::redirects::{MapTarget, Matching, moved, path_of, redirects_csv};
use crate::substack::ImportedPost;

/// The redirect map's file, from the site folder, for the host or the
/// redirect manager.
pub const REDIRECTS_CSV: &str = "import/substack-redirects.csv";

/// One old address and where it leads.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MapEntry {
    /// The whole old address, such as `https://rye.substack.com/p/rye`.
    pub from: AbsoluteUrl,
    /// Where it leads.
    pub to: MapTarget,
    /// Substack's number for the post.
    pub post: String,
}

impl MapEntry {
    /// Whether only Substack can answer this address, because it is on a
    /// `substack.com` subdomain.
    pub fn on_substack(&self) -> bool {
        let host = self.from.host();
        host == "substack.com" || host.ends_with(".substack.com")
    }
}

/// The whole map: for each published post, in the export's order, its old
/// address. Drafts were never public, so they have none.
pub(crate) fn redirect_map(posts: &[ImportedPost], site: &Config) -> Vec<MapEntry> {
    let origin = site.address().as_str().trim_end_matches('/');
    let mut map: Vec<MapEntry> = Vec::new();
    for post in posts {
        let (Some(from), Some(new)) = (&post.old, post.address.as_deref()) else {
            continue;
        };
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
            post: post.id.clone(),
        });
    }
    map
}

/// The map as CSV, every entry that redirects. An address on the new
/// site's own host is written as its path, which the host's rules match;
/// any other is written whole.
pub(crate) fn substack_csv(map: &[MapEntry], site: &Config) -> String {
    redirects_csv(map.iter().map(|entry| {
        let from = if entry.from.host() == site.address().host() {
            path_of(&entry.from)
        } else {
            entry.from.as_str()
        };
        (from, &entry.to, Matching::Exact)
    }))
}
