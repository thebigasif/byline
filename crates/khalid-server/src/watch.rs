//! Watching the site folder, and rendering the pages again when a file
//! changes: by file events where the system has them, and by looking at
//! the folder where it does not.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::oneshot;
use tokio::time::MissedTickBehavior;

use crate::watch_events::{Events, follow_events};
use crate::{Background, LivePages, Render};

/// The folders at the top of a site that Khalid writes itself: the
/// database and caches, and the static export. A change there is never an
/// edit.
const OWN_FOLDERS: [&str; 2] = ["data", "public"];

/// Every file under the site folder, with its size and when it last
/// changed.
pub(crate) type Snapshot = BTreeMap<PathBuf, (u64, Option<SystemTime>)>;

/// What the site folder held at one moment: every file [`watch`] looks at,
/// with its size and when it last changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderLook(Snapshot);

impl FolderLook {
    /// Looks at every file under `site` now. Take it before the pages are
    /// first rendered and hand it to [`watch`], so a file saved during the
    /// render or before the watch starts is still seen as a change.
    ///
    /// Reads the folder, so call it before the async runtime starts or on
    /// a blocking thread (RULES.md BL-RUN-7).
    pub fn take(site: &Path) -> Self {
        Self(scan(site))
    }
}

/// Renders the pages again with `renderer` and serves them through `live`
/// whenever a file under `site` is added, removed or changed after the
/// look `since`.
///
/// It listens for the system's file events, so a save shows at once
/// (CTO answers 2026-10-27, section 1). Where file events cannot be had,
/// such as on some network drives, it falls back to [`poll`], looking at
/// the folder each `every`.
///
/// Hidden files and folders, such as `.git` and editor swap files, and
/// the `data/` and `public/` folders Khalid writes itself, are left out,
/// so the server's own writes never cause a rebuild. A render that fails
/// is logged and the pages readers see stay as they were; the next save
/// tries again. Must be called inside the tokio runtime.
pub fn watch<R: Render + Clone>(
    site: &Path,
    renderer: &R,
    live: &LivePages,
    since: FolderLook,
    every: Duration,
) -> Background {
    let (site, renderer, live) = (Arc::<Path>::from(site), renderer.clone(), live.clone());
    Background::spawn(move |stopped| async move {
        let root = Arc::clone(&site);
        // Setting the watch up walks the folders, so it runs off the
        // runtime (RULES.md BL-RUN-7).
        match tokio::task::spawn_blocking(move || Events::start(&root)).await {
            Ok(Ok(events)) => follow_events(site, renderer, live, since.0, events, stopped).await,
            Ok(Err(error)) => {
                tracing::info!(
                    %error,
                    "file events are not available here, so the server looks at the folder for changes"
                );
                poll_until_stopped(site, renderer, live, since.0, every, stopped).await;
            }
            Err(_) => poll_until_stopped(site, renderer, live, since.0, every, stopped).await,
        }
    })
}

/// Like [`watch`], but only by looking at every file under `site` each
/// `every` and comparing sizes and change times with the look before.
///
/// This needs no file-event service, so it works the same on every
/// system, in containers and on network drives. [`watch`] falls back to
/// it. Must be called inside the tokio runtime.
pub fn poll<R: Render + Clone>(
    site: &Path,
    renderer: &R,
    live: &LivePages,
    since: FolderLook,
    every: Duration,
) -> Background {
    let (site, renderer, live) = (Arc::from(site), renderer.clone(), live.clone());
    Background::spawn(move |stopped| {
        poll_until_stopped(site, renderer, live, since.0, every, stopped)
    })
}

async fn poll_until_stopped<R: Render + Clone>(
    site: Arc<Path>,
    renderer: R,
    live: LivePages,
    mut seen: Snapshot,
    every: Duration,
    mut stopped: oneshot::Receiver<()>,
) {
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
        refresh(&live, &renderer).await;
    }
}

/// Renders the pages again and serves them, or logs why it could not and
/// keeps the pages readers see.
pub(crate) async fn refresh<R: Render + Clone>(live: &LivePages, renderer: &R) {
    match live.refresh(renderer).await {
        Ok(()) => tracing::info!("a file changed, so the pages were rendered again"),
        Err(error) => tracing::warn!(
            ?error,
            "a file changed but the pages could not be rendered, so readers keep the last good pages"
        ),
    }
}

/// Reads the folder on a blocking thread (RULES.md BL-RUN-7). `None` when
/// that thread stopped, which skips one look.
pub(crate) async fn snapshot(site: &Arc<Path>) -> Option<Snapshot> {
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

/// Whether `path`, under `root`, is hidden or sits in a hidden folder, or
/// in one of Khalid's own folders at the top of the site. A path outside
/// `root` is never left out.
pub(crate) fn is_left_out(root: &Path, path: &Path) -> bool {
    let Ok(below) = path.strip_prefix(root) else {
        return false;
    };
    let mut names = below.components().filter_map(|part| match part {
        Component::Normal(name) => name.to_str(),
        _ => None,
    });
    let Some(top) = names.next() else {
        return false;
    };
    OWN_FOLDERS.contains(&top) || top.starts_with('.') || names.any(|name| name.starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_and_own_files_are_left_out() {
        let root = Path::new("/srv/site");
        let left_out = |path: &str| is_left_out(root, Path::new(path));
        assert!(left_out("/srv/site/data/site.db"));
        assert!(left_out("/srv/site/public/index.html"));
        assert!(left_out("/srv/site/.git/index"));
        assert!(left_out("/srv/site/content/en/posts/.what.md.swp"));
        assert!(!left_out("/srv/site/content/en/posts/what.md"));
        assert!(!left_out("/srv/site/content/en/data/what.md"));
        assert!(!left_out("/srv/site/site.toml"));
        assert!(!left_out("/elsewhere/data/site.db"));
    }
}
