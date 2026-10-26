//! Watching the site folder, and rendering the pages again when a file
//! changes.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::oneshot;
use tokio::time::MissedTickBehavior;

use crate::{Background, LivePages, Render};

/// The folders at the top of a site that Khalid writes itself: the
/// database and caches, and the static export. A change there is never an
/// edit.
const OWN_FOLDERS: [&str; 2] = ["data", "public"];

/// Every file under the site folder, with its size and when it last
/// changed.
type Snapshot = BTreeMap<PathBuf, (u64, Option<SystemTime>)>;

/// Looks at every file under `site` each `every`, and when one is added,
/// removed or changed, renders the pages again with `renderer` and serves
/// them through `live`.
///
/// Hidden files and folders, such as `.git` and editor swap files, and
/// the `data/` and `public/` folders Khalid writes itself, are left out,
/// so the server's own writes never cause a rebuild. A render that fails
/// is logged and the pages readers see stay as they were; the next save
/// tries again. Must be called inside the tokio runtime.
///
/// Comparing sizes and change times needs no file-event service, so it
/// works the same on every system, in containers and on network drives.
pub fn watch<R: Render + Clone>(
    site: &Path,
    renderer: &R,
    live: &LivePages,
    every: Duration,
) -> Background {
    let (site, renderer, live) = (Arc::from(site), renderer.clone(), live.clone());
    Background::spawn(move |stopped| watch_until_stopped(site, renderer, live, every, stopped))
}

async fn watch_until_stopped<R: Render + Clone>(
    site: Arc<Path>,
    renderer: R,
    live: LivePages,
    every: Duration,
    mut stopped: oneshot::Receiver<()>,
) {
    let mut seen = snapshot(&site).await.unwrap_or_default();
    let mut ticks = tokio::time::interval(every);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = &mut stopped => return,
            _ = ticks.tick() => {}
        }
        let Some(now) = snapshot(&site).await else {
            continue;
        };
        if now == seen {
            continue;
        }
        seen = now;
        match live.refresh(&renderer).await {
            Ok(()) => tracing::info!("a file changed, so the pages were rendered again"),
            Err(error) => tracing::warn!(
                ?error,
                "a file changed but the pages could not be rendered, so readers keep the last good pages"
            ),
        }
    }
}

/// Reads the folder on a blocking thread (RULES.md BL-RUN-7). `None` when
/// that thread stopped, which skips one look.
async fn snapshot(site: &Arc<Path>) -> Option<Snapshot> {
    let site = Arc::clone(site);
    tokio::task::spawn_blocking(move || scan(&site)).await.ok()
}

/// Every file under `root` but the ones [`is_left_out`] names. A file that
/// disappears while it is read is simply not listed.
fn scan(root: &Path) -> Snapshot {
    let mut seen = Snapshot::new();
    let mut folders = vec![root.to_owned()];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if is_left_out(root, &path) {
                continue;
            }
            // `DirEntry::metadata` does not follow a link, so a link to a
            // folder above can never make the walk loop.
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                folders.push(path);
            } else {
                seen.insert(path, (meta.len(), meta.modified().ok()));
            }
        }
    }
    seen
}

/// Whether `path` is hidden, or one of Khalid's own folders at the top of
/// the site.
fn is_left_out(root: &Path, path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with('.') || (path.parent() == Some(root) && OWN_FOLDERS.contains(&name))
}
