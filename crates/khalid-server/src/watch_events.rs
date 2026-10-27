//! The system's file events for the site folder, through `notify`, and
//! what the watch does with them: the saved pages first, then the rest.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::event::{AccessKind, AccessMode};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, oneshot};

use crate::watch::{Snapshot, is_left_out, snapshot};
use crate::{LivePages, Render};

/// How long the watch waits after the first event of a save for the rest
/// of it. An editor that saves through a new file and a rename makes
/// several events in well under this, and they cost one render.
const SETTLE: Duration = Duration::from_millis(10);

/// What one file event says changed.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Change {
    /// These files, relative to the site folder.
    Files(Vec<PathBuf>),
    /// Something, but the event cannot say what: an error, lost events, or
    /// a path outside the folder as the watch knows it.
    Unknown,
}

/// The system watching the site folder, and what it reports each time a
/// file that counts changes.
pub(crate) struct Events {
    /// Kept alive for as long as the events are wanted: dropping it ends
    /// the watch.
    _watcher: RecommendedWatcher,
    changed: mpsc::UnboundedReceiver<Change>,
}

impl Events {
    /// Asks the system for events on every file under `root`. Walks the
    /// folders to do so, so it runs on a blocking thread (RULES.md
    /// BL-RUN-7).
    ///
    /// # Errors
    ///
    /// Returns the [`notify::Error`] when this system or this folder has
    /// no file events, such as on some network drives.
    pub(crate) fn start(root: &Path) -> Result<Self, notify::Error> {
        let (signal, changed) = mpsc::unbounded_channel();
        // Event paths may name the folder as given, made absolute, or with
        // every link resolved, as on macOS, so it is matched in each form.
        let given = root.to_owned();
        let absolute = std::path::absolute(root).unwrap_or_else(|_| root.to_owned());
        let resolved = fs::canonicalize(root).unwrap_or_else(|_| root.to_owned());
        let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            if let Some(change) = change(&[&given, &absolute, &resolved], event) {
                // The receiver is gone only once the watch has stopped.
                let _ = signal.send(change);
            }
        })?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            changed,
        })
    }

    /// Waits for the next change, then [`SETTLE`]s and takes every change
    /// reported meanwhile: the files saved, or `None` when one change could
    /// not say which. Gives back nothing once the events have ended.
    async fn next_save(&mut self) -> Option<Option<BTreeSet<PathBuf>>> {
        let first = self.changed.recv().await?;
        tokio::time::sleep(SETTLE).await;
        let mut saved = Some(BTreeSet::new());
        let mut take = |change| match (change, saved.as_mut()) {
            (Change::Files(files), Some(all)) => all.extend(files),
            _ => saved = None,
        };
        take(first);
        while let Ok(change) = self.changed.try_recv() {
            take(change);
        }
        Some(saved)
    }
}

/// What `event` in the site folder, named in any of the forms `site`, says
/// changed, or `None` when nothing that can change a page did. Reading a
/// file is no change, since every render reads them all, and neither is a
/// file [`is_left_out`] names. An error or a lost event might hide a
/// change, so it is an unknown one.
fn change(site: &[&Path], event: notify::Result<Event>) -> Option<Change> {
    let Ok(event) = event else {
        return Some(Change::Unknown);
    };
    if event.need_rescan() {
        return Some(Change::Unknown);
    }
    if matches!(event.kind, EventKind::Access(kind) if kind != AccessKind::Close(AccessMode::Write))
    {
        return None;
    }
    let mut files = Vec::new();
    for path in &event.paths {
        if site.iter().any(|root| is_left_out(root, path)) {
            continue;
        }
        let below = site.iter().find_map(|root| path.strip_prefix(root).ok());
        match below {
            Some(below) => files.push(below.to_owned()),
            None => return Some(Change::Unknown),
        }
    }
    (!files.is_empty()).then_some(Change::Files(files))
}

/// Follows `events` until `stopped`. After each save it shows the saved
/// files' own pages at once, when the renderer can make them alone, and
/// asks for a full render beside it, so a slow full render never holds
/// the next save back. A file saved between the look `since` and the start
/// of the events made no event, so the folder is compared with `since`
/// once first.
pub(crate) async fn follow_events<R: Render + Clone>(
    site: Arc<Path>,
    renderer: R,
    live: LivePages,
    since: Snapshot,
    mut events: Events,
    mut stopped: oneshot::Receiver<()>,
) {
    let (ask, asked) = mpsc::channel(1);
    let full = tokio::spawn(render_when_asked(live.clone(), renderer.clone(), asked));
    if snapshot(&site).await.is_some_and(|now| now != since) {
        let _ = ask.try_send(());
    }
    loop {
        let saved = tokio::select! {
            _ = &mut stopped => break,
            saved = events.next_save() => saved,
        };
        let Some(saved) = saved else {
            break;
        };
        if let Some(files) = saved {
            show_saved(&live, &renderer, files.into_iter().collect()).await;
        }
        // A full render already asked for takes this save in too.
        let _ = ask.try_send(());
    }
    // The full render in progress finishes before the watch counts as
    // stopped, so the caller can close the database after it.
    drop(ask);
    let _ = full.await;
}

/// Renders the pages of the saved `files` alone on a blocking thread and
/// serves them. Logs and leaves the pages as they are when the renderer
/// cannot, since the full render follows.
async fn show_saved<R: Render + Clone>(live: &LivePages, renderer: &R, files: Vec<PathBuf>) {
    let renderer = renderer.clone();
    let outcome = tokio::task::spawn_blocking(move || renderer.render_saved(&files)).await;
    match outcome {
        Ok(Ok(Some(pages))) if !pages.is_empty() => live.show_saved(pages),
        Ok(Ok(_)) => {}
        Ok(Err(error)) => tracing::debug!(
            ?error,
            "the saved page could not be rendered alone, so it waits for the full render"
        ),
        Err(error) => tracing::debug!(?error, "the thread rendering the saved page stopped"),
    }
}

/// Renders every page each time it is `asked`, until the asking side is
/// gone. When the pages were replaced during a render, by the pages of a
/// newer save, the render is stale and runs again.
async fn render_when_asked<R: Render + Clone>(
    live: LivePages,
    renderer: R,
    mut asked: mpsc::Receiver<()>,
) {
    while asked.recv().await.is_some() {
        loop {
            match live.refresh_unless_replaced(&renderer).await {
                Ok(true) => tracing::info!("a file changed, so the pages were rendered again"),
                Ok(false) => continue,
                Err(error) => tracing::warn!(
                    ?error,
                    "a file changed but the pages could not be rendered, so readers keep the last good pages"
                ),
            }
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use notify::event::{CreateKind, ModifyKind};

    use super::*;

    /// What `site` makes of one event of `kind` on `path`.
    fn seen(site: &[&Path], kind: EventKind, path: &str) -> Option<Change> {
        change(site, Ok(Event::new(kind).add_path(PathBuf::from(path))))
    }

    #[test]
    fn only_writes_to_files_that_count_are_changes() {
        let site: &[&Path] = &[Path::new("/srv/site"), Path::new("/private/srv/site")];
        let post = "/srv/site/content/en/posts/what.md";
        let relative = Some(Change::Files(vec![PathBuf::from(
            "content/en/posts/what.md",
        )]));
        let write = EventKind::Modify(ModifyKind::Any);
        let created = EventKind::Create(CreateKind::File);
        let written = EventKind::Access(AccessKind::Close(AccessMode::Write));
        let read = EventKind::Access(AccessKind::Close(AccessMode::Read));
        assert_eq!(seen(site, write, post), relative);
        assert_eq!(seen(site, created, post), relative);
        assert_eq!(seen(site, written, post), relative);
        let resolved = "/private/srv/site/content/en/posts/what.md";
        assert_eq!(seen(site, write, resolved), relative);
        assert_eq!(seen(site, read, post), None);
        assert_eq!(seen(site, write, "/srv/site/data/site.db-wal"), None);
        assert_eq!(
            seen(site, write, "/private/srv/site/data/site.db-wal"),
            None
        );
    }

    #[test]
    fn a_change_the_event_cannot_name_is_unknown() {
        let site: &[&Path] = &[Path::new("/srv/site")];
        let error = Err(notify::Error::generic("queue overflow"));
        assert_eq!(change(site, error), Some(Change::Unknown));
        let write = EventKind::Modify(ModifyKind::Any);
        assert_eq!(
            seen(site, write, "/mnt/site/site.toml"),
            Some(Change::Unknown)
        );
    }
}
