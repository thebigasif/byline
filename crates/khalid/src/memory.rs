//! Handing memory back to the system once `khalid serve` has gone quiet.
//!
//! mimalloc, the binary's allocator, returns freed memory to the system
//! one second after it is freed, but only when a thread later frees a
//! whole page or ends. A server with no visitors does neither, so the
//! memory a render freed would stay resident until the next request
//! (docs/standards/performance.md, "Allocator settings").

use std::hint::black_box;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use khalid_server::Replacements;
use tokio::sync::Notify;

/// How long the server waits after its last heavy work before handing
/// memory back: past mimalloc's purge delay of one second, so everything
/// freed by then is due.
const QUIET: Duration = Duration::from_secs(2);

/// Hands freed memory back a moment after heavy work: a render, which
/// replaces the served pages, or a rebuild of the Content API's store and
/// index. Cloning shares it.
#[derive(Debug, Clone, Default)]
pub(crate) struct HandBack(Arc<Notify>);

impl HandBack {
    /// Says that heavy work other than a page render finished.
    pub(crate) fn after_work(&self) {
        self.0.notify_one();
    }

    /// Starts handing memory back [`QUIET`] after each burst of work: the
    /// pages of `replacements` replaced, or [`HandBack::after_work`].
    /// Must be called inside the tokio runtime; the job ends with it.
    pub(crate) fn start(&self, replacements: Replacements) {
        self.start_with(replacements, QUIET, end_a_thread);
    }

    fn start_with(
        &self,
        mut replacements: Replacements,
        quiet: Duration,
        hand_back: impl Fn() + Send + 'static,
    ) {
        let work = Arc::clone(&self.0);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    () = replacements.next() => {}
                    () = work.notified() => {}
                }
                // More work before the quiet ends starts the wait again.
                loop {
                    tokio::select! {
                        () = replacements.next() => {}
                        () = work.notified() => {}
                        () = tokio::time::sleep(quiet) => break,
                    }
                }
                hand_back();
            }
        });
    }
}

/// Asks Linux to back this process's memory with 4 KB pages only, never
/// 2 MB huge pages. Where the system hands out huge pages unasked, as on
/// CI's runners, the kernel fills whole 2 MB pages around the little
/// memory an idle server still uses, and each counts whole. Call it before
/// the content is read. Elsewhere it does nothing.
pub(crate) fn small_pages_only() {
    #[cfg(target_os = "linux")]
    if let Err(error) = rustix::thread::disable_transparent_huge_pages(true) {
        tracing::debug!(%error, "huge pages stay on, so idle memory may be higher");
    }
}

/// Ends a short-lived thread. As a thread ends, mimalloc collects its heap
/// and returns every freed page whose delay has passed, from every thread.
/// The thread allocates one byte so that it has a heap to collect.
fn end_a_thread() {
    let ended = thread::Builder::new()
        .name("khalid-hand-back".to_owned())
        .spawn(|| drop(black_box(Box::new(0_u8))));
    if let Err(error) = ended {
        tracing::debug!(%error, "freed memory is handed back after the next render instead");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use khalid_server::{LivePages, Pages};

    use super::*;

    /// Long enough that back-to-back calls on a busy machine fall inside it.
    const TEST_QUIET: Duration = Duration::from_millis(300);

    fn started() -> (LivePages, HandBack, Arc<AtomicUsize>) {
        let live = LivePages::new(Pages::first(Vec::new()).unwrap());
        let (hand_back, count) = (HandBack::default(), Arc::new(AtomicUsize::new(0)));
        let counted = Arc::clone(&count);
        hand_back.start_with(live.replacements(), TEST_QUIET, move || {
            counted.fetch_add(1, Ordering::SeqCst);
        });
        (live, hand_back, count)
    }

    async fn after_quiet(count: &AtomicUsize) -> usize {
        tokio::time::sleep(TEST_QUIET * 3).await;
        count.load(Ordering::SeqCst)
    }

    #[tokio::test]
    async fn nothing_is_handed_back_before_any_work() {
        let (_live, _hand_back, count) = started();
        assert_eq!(after_quiet(&count).await, 0);
    }

    #[tokio::test]
    async fn a_burst_of_replacements_hands_back_once_after_the_quiet() {
        let (live, _hand_back, count) = started();
        tokio::task::yield_now().await;
        live.show_saved(Vec::new());
        live.show_saved(Vec::new());
        live.show_saved(Vec::new());
        assert_eq!(count.load(Ordering::SeqCst), 0, "not before the quiet");
        assert_eq!(after_quiet(&count).await, 1);
        live.show_saved(Vec::new());
        assert_eq!(after_quiet(&count).await, 2, "each burst once");
    }

    #[tokio::test]
    async fn work_outside_a_render_hands_back_too() {
        let (_live, hand_back, count) = started();
        hand_back.after_work();
        assert_eq!(after_quiet(&count).await, 1);
    }

    #[test]
    fn ending_a_thread_returns() {
        end_a_thread();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn huge_pages_are_off_once_asked() {
        small_pages_only();
        assert_eq!(
            rustix::thread::transparent_huge_pages_are_disabled().ok(),
            Some(true)
        );
    }
}
