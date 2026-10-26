//! The system's file events for the site folder, through `notify`.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use notify::event::{AccessKind, AccessMode};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, oneshot};

use crate::watch::{Snapshot, is_left_out, refresh, snapshot};
use crate::{LivePages, Render};

/// How long the watch waits after the first event of a save for the rest
/// of it. An editor that saves through a new file and a rename makes
/// several events in well under this, and they cost one render.
const SETTLE: Duration = Duration::from_millis(10);

/// The system watching the site folder, and the signal it sends each time
/// a file that counts changes.
pub(crate) struct Events {
    /// Kept alive for as long as the events are wanted: dropping it ends
    /// the watch.
    _watcher: RecommendedWatcher,
    changed: mpsc::UnboundedReceiver<()>,
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
        // Some systems, such as macOS, name event paths with every link
        // resolved, so the folder is matched in that form too.
        let site = fs::canonicalize(root).unwrap_or_else(|_| root.to_owned());
        let given = root.to_owned();
        let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            if counts(&[&site, &given], event) {
                // The receiver is gone only once the watch has stopped.
                let _ = signal.send(());
            }
        })?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            changed,
        })
    }
}

/// Whether `event` in the site folder, named in any of the forms `site`,
/// can change a page. Reading a file is no change, since every render
/// reads them all, and neither is a file [`is_left_out`] names. An error
/// or a lost event might hide a change, so it counts.
fn counts(site: &[&Path], event: notify::Result<Event>) -> bool {
    let Ok(event) = event else {
        return true;
    };
    if event.need_rescan() {
        return true;
    }
    if matches!(event.kind, EventKind::Access(kind) if kind != AccessKind::Close(AccessMode::Write))
    {
        return false;
    }
    event
        .paths
        .iter()
        .any(|path| !site.iter().any(|root| is_left_out(root, path)))
}

/// Renders the pages again after each change `events` report, until
/// `stopped`. A file saved between the look `since` and the start of the
/// events made no event, so the folder is compared with `since` once
/// first.
pub(crate) async fn follow_events<R: Render + Clone>(
    site: Arc<Path>,
    renderer: R,
    live: LivePages,
    since: Snapshot,
    mut events: Events,
    mut stopped: oneshot::Receiver<()>,
) {
    if snapshot(&site).await.is_some_and(|now| now != since) {
        refresh(&live, &renderer).await;
    }
    loop {
        tokio::select! {
            _ = &mut stopped => return,
            changed = events.changed.recv() => {
                if changed.is_none() {
                    return;
                }
            }
        }
        tokio::time::sleep(SETTLE).await;
        while events.changed.try_recv().is_ok() {}
        refresh(&live, &renderer).await;
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use notify::event::{CreateKind, ModifyKind};

    use super::*;

    fn event(kind: EventKind, path: &str) -> Event {
        Event::new(kind).add_path(PathBuf::from(path))
    }

    #[test]
    fn only_writes_to_files_that_count_cause_a_render() {
        let site: &[&Path] = &[Path::new("/private/srv/site"), Path::new("/srv/site")];
        let post = "/srv/site/content/en/posts/what.md";
        let write = EventKind::Modify(ModifyKind::Any);
        let created = EventKind::Create(CreateKind::File);
        let written = EventKind::Access(AccessKind::Close(AccessMode::Write));
        let read = EventKind::Access(AccessKind::Close(AccessMode::Read));
        assert!(counts(site, Ok(event(write, post))));
        assert!(counts(site, Ok(event(created, post))));
        assert!(counts(site, Ok(event(written, post))));
        assert!(!counts(site, Ok(event(read, post))));
        assert!(!counts(
            site,
            Ok(event(write, "/srv/site/data/site.db-wal"))
        ));
        assert!(!counts(
            site,
            Ok(event(write, "/private/srv/site/data/site.db-wal"))
        ));
        assert!(counts(site, Err(notify::Error::generic("queue overflow"))));
    }
}
