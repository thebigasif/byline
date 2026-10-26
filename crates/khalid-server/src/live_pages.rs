//! The page set readers see now, replaced as a whole when the site changes.

use std::sync::{Arc, PoisonError, RwLock};

use crate::{Pages, Render, ServerError, render_pages};

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
}

impl From<Pages> for LivePages {
    fn from(pages: Pages) -> Self {
        Self::new(pages)
    }
}
