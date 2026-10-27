//! The page set readers see now, replaced as a whole when the site changes.

use std::sync::{Arc, PoisonError, RwLock};

use crate::{Page, Pages, Render, ServerError, render_pages};

/// The pages being served, shared by every request and replaced as a whole
/// after a change, so a reader never sees half a rebuild. Cloning shares
/// the same set.
#[derive(Debug, Clone)]
pub struct LivePages(Arc<RwLock<Arc<Pages>>>);

impl LivePages {
    /// Serves `pages` until they are replaced.
    pub fn new(pages: Pages) -> Self {
        Self(Arc::new(RwLock::new(Arc::new(pages))))
    }

    /// The pages being served now.
    pub fn current(&self) -> Arc<Pages> {
        // A panic cannot happen while the lock is held, since holding it
        // only reads or swaps an `Arc`, so a poisoned lock still holds a
        // whole page set.
        let pages = self.0.read().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(&pages)
    }

    /// Renders the site again with `renderer` and serves the new pages.
    /// Pages whose bytes did not change keep their Brotli copy.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`render_pages`]. The pages being served do
    /// not change, so a broken edit never takes the site down.
    pub async fn refresh<R: Render + Clone>(&self, renderer: &R) -> Result<(), ServerError> {
        let pages = render_pages(renderer, Some(self.current())).await?;
        let mut slot = self.0.write().unwrap_or_else(PoisonError::into_inner);
        *slot = Arc::new(pages);
        Ok(())
    }

    /// Serves the `saved` pages in place of the pages at their addresses,
    /// and every other page as it is (see [`Pages::with_saved`]).
    pub fn show_saved(&self, saved: Vec<Page>) {
        let mut slot = self.0.write().unwrap_or_else(PoisonError::into_inner);
        *slot = Arc::new(slot.with_saved(saved));
    }

    /// Renders the site again with `renderer`, like [`LivePages::refresh`],
    /// but serves the new pages only when nothing replaced the pages while
    /// it rendered, such as [`LivePages::show_saved`] after a newer save.
    /// Says whether the new pages are served.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`render_pages`]; the pages being served do
    /// not change.
    pub async fn refresh_unless_replaced<R: Render + Clone>(
        &self,
        renderer: &R,
    ) -> Result<bool, ServerError> {
        let before = self.current();
        let pages = render_pages(renderer, Some(Arc::clone(&before))).await?;
        Ok(self.replace_if_current(&before, pages))
    }

    /// Adds a Brotli copy to every page being served, on a blocking thread,
    /// then serves the compressed set. Start the server with
    /// [`Pages::plain`] and call this once it listens, so the first reader
    /// does not wait for compression.
    ///
    /// When the pages were replaced meanwhile, by an edit, the compressed
    /// set is dropped: the newer pages were compressed when they were built.
    ///
    /// # Errors
    ///
    /// Returns [`ServerError::Worker`] when the compressing thread stopped.
    /// The plain pages keep being served.
    pub async fn compress(&self) -> Result<(), ServerError> {
        let plain = self.current();
        let work = Arc::clone(&plain);
        let compressed = tokio::task::spawn_blocking(move || work.compressed())
            .await
            .map_err(|source| ServerError::Worker { source })?;
        self.replace_if_current(&plain, compressed);
        Ok(())
    }

    /// Serves `pages` in place of `expected`, unless something else
    /// replaced `expected` first. Says whether `pages` are served now.
    pub fn replace_if_current(&self, expected: &Arc<Pages>, pages: Pages) -> bool {
        let mut slot = self.0.write().unwrap_or_else(PoisonError::into_inner);
        if !Arc::ptr_eq(&slot, expected) {
            return false;
        }
        *slot = Arc::new(pages);
        true
    }
}

impl From<Pages> for LivePages {
    fn from(pages: Pages) -> Self {
        Self::new(pages)
    }
}
