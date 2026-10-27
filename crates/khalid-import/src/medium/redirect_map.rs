//! Where every old address of a Medium post leads in Khalid, each once
//! (ADR "an import maps every old address"). A Medium post answers at its
//! own address, on medium.com or on a publication's domain, and at
//! Medium's short address `https://medium.com/p/<id>`.

use khalid_core::{AbsoluteUrl, Config};

use crate::medium::{Export, ImportedPost};
use crate::redirects::{MapTarget, Matching, moved, path_of, redirects_csv};

/// The redirect map's file, from the site folder, for the host or the
/// redirect manager.
pub const REDIRECTS_CSV: &str = "import/medium-redirects.csv";

/// Medium's own host, whose addresses only Medium can answer.
const MEDIUM: &str = "medium.com";

/// One old address and where it leads.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MapEntry {
    /// The whole old address, since a Medium post may have lived on
    /// medium.com or on a publication's own domain.
    pub from: AbsoluteUrl,
    /// Where it leads.
    pub to: MapTarget,
    /// Medium's id for the post.
    pub post: String,
}

impl MapEntry {
    /// Whether only Medium can answer this address, because it is on
    /// medium.com or one of its subdomains.
    pub fn on_medium(&self) -> bool {
        let host = self.from.host();
        host == MEDIUM || host.ends_with(".medium.com")
    }
}

/// The whole map: for each published post, in file name order, its own
/// address and then its short one. An address already listed is not
/// listed again. Drafts were never public, so they have none.
pub(crate) fn redirect_map(
    export: &Export,
    posts: &[ImportedPost],
    site: &Config,
) -> Vec<MapEntry> {
    let origin = site.address().as_str().trim_end_matches('/');
    let mut map: Vec<MapEntry> = Vec::new();
    for post in posts {
        let (Some(new), Some(medium)) = (
            post.address.as_deref(),
            export.posts.iter().find(|medium| medium.id == post.id),
        ) else {
            continue;
        };
        let short = AbsoluteUrl::parse(&format!("https://{MEDIUM}/p/{}", post.id)).ok();
        for from in medium.canonical.iter().cloned().chain(short) {
            if map.iter().any(|entry| entry.from == from) {
                continue;
            }
            let same = from.host() == site.address().host() && path_of(&from) == new;
            let to = if same {
                MapTarget::Kept
            } else {
                moved(&format!("{origin}{new}"))
            };
            map.push(MapEntry {
                from,
                to,
                post: post.id.clone(),
            });
        }
    }
    map
}

/// The map as CSV, every entry that redirects. An address on the new
/// site's own host is written as its path, which the host's rules match;
/// any other is written whole.
pub(crate) fn medium_csv(map: &[MapEntry], site: &Config) -> String {
    let rows: Vec<(&str, &MapTarget)> = map
        .iter()
        .map(|entry| {
            let from = if entry.from.host() == site.address().host() {
                path_of(&entry.from)
            } else {
                entry.from.as_str()
            };
            (from, &entry.to)
        })
        .collect();
    redirects_csv(
        rows.into_iter()
            .map(|(from, to)| (from, to, Matching::Exact)),
    )
}
