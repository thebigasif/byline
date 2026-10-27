//! The page set readers see now, replaced as a whole when the site changes.

use std::sync::Arc;

use tokio::sync::watch;

use crate::{Page, Pages, Render, ServerError, render_pages};

/// The pages being served, shared by every request and replaced as a whole
/// after a change, so a reader never sees half a rebuild. Cloning shares
/// the same set.
#[derive(Debug, Clone)]
pub struct LivePages(Arc<watch::Sender<Arc<Pages>>>);

impl LivePages {
    /// Serves `pages` until they are replaced.
    pub fn new(pages: Pages) -> Self {
        Self(Arc::new(watch::Sender::new(Arc::new(pages))))
    }

    /// The pages being served now.
    pub fn current(&self) -> Arc<Pages> {
        Arc::clone(&self.0.borrow())
    }

    /// The pages being served once they are every page of the site: at
    /// once, unless the server started with [`Pages::first`] and the rest
    /// is still being rendered.
    pub async fn whole(&self) -> Arc<Pages> {
        let mut served = self.0.subscribe();
        match served.wait_for(|pages| pages.is_whole()).await {
            Ok(pages) => Arc::clone(&pages),
            // The sender lives as long as `self`, so it cannot be closed.
            Err(_) => self.current(),
        }
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
        self.0.send_replace(Arc::new(pages));
        Ok(())
    }

    /// Serves the `saved` pages in place of the pages at their addresses,
    /// and every other page as it is (see [`Pages::with_saved`]).
    pub fn show_saved(&self, saved: Vec<Page>) {
        self.0
            .send_modify(|pages| *pages = Arc::new(pages.with_saved(saved)));
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
        self.0.send_if_modified(|served| {
            if !Arc::ptr_eq(served, expected) {
                return false;
            }
            *served = Arc::new(pages);
            true
        })
    }
}

impl From<Pages> for LivePages {
    fn from(pages: Pages) -> Self {
        Self::new(pages)
    }
}
