//! Where pages come from: the site's renderer, run off the async runtime.

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use bytes::Bytes;

use crate::{Moved, Page, Pages, ServerError};

/// Renders every page of a site from its files.
///
/// The server does not know markdown, templates or the content tree. The
/// wiring in the binary implements this with the content, routes, render
/// and SEO crates, and the server calls it at start and after each change.
/// It runs on a blocking thread, so it may read files and do CPU work
/// (RULES.md BL-RUN-7).
pub trait Render: Send + Sync + 'static {
    /// Why the site could not be rendered.
    type Error: Error + Send + Sync + 'static;

    /// Every page of the site, and the not-found page.
    ///
    /// # Errors
    ///
    /// Returns the renderer's error when a file cannot be read or a page
    /// cannot be rendered.
    fn render(&self) -> Result<Rendered, Self::Error>;

    /// The pages of the saved `files` alone, given relative to the site
    /// folder, rendered quickly so the watch can show them before the full
    /// render finishes. `None` when the renderer cannot tell which pages
    /// those files make, and the default: then only the full render runs.
    ///
    /// The full render always follows, so a page that lists the saved
    /// files, such as the home page, catches up a moment later.
    ///
    /// # Errors
    ///
    /// Returns the renderer's error when a page cannot be rendered. The
    /// watch then waits for the full render.
    fn render_saved(&self, files: &[PathBuf]) -> Result<Option<Vec<Page>>, Self::Error> {
        let _ = files;
        Ok(None)
    }
}

impl<R: Render + ?Sized> Render for Arc<R> {
    type Error = R::Error;

    fn render(&self) -> Result<Rendered, Self::Error> {
        R::render(self)
    }

    fn render_saved(&self, files: &[PathBuf]) -> Result<Option<Vec<Page>>, Self::Error> {
        R::render_saved(self, files)
    }
}

/// What a [`Render`] gives back: every page, the HTML for an address with
/// nothing behind it, and the addresses that moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pages: Vec<Page>,
    not_found: Bytes,
    moves: Vec<Moved>,
}

impl Rendered {
    /// The rendered `pages` and the `not_found` page, with no moves.
    pub fn new(pages: Vec<Page>, not_found: impl Into<Bytes>) -> Self {
        Self {
            pages,
            not_found: not_found.into(),
            moves: Vec::new(),
        }
    }

    /// The same, with `moves` answered by 301 where no page sits.
    #[must_use]
    pub fn with_moves(self, moves: Vec<Moved>) -> Self {
        Self { moves, ..self }
    }

    /// Splits into the pages, the not-found page and the moves.
    pub(crate) fn into_parts(self) -> (Vec<Page>, Bytes, Vec<Moved>) {
        (self.pages, self.not_found, self.moves)
    }
}

/// Renders the site with `renderer` on a blocking thread and builds the
/// page set, reusing the Brotli copy of every page whose bytes are the same
/// as in `previous`.
///
/// # Errors
///
/// Returns [`ServerError::Render`] when the renderer fails,
/// [`ServerError::DuplicatePage`] when two pages claim one address, and
/// [`ServerError::Worker`] when the rendering thread stopped.
pub async fn render_pages<R: Render + Clone>(
    renderer: &R,
    previous: Option<Arc<Pages>>,
) -> Result<Pages, ServerError> {
    let renderer = renderer.clone();
    let work = move || {
        let output = renderer.render().map_err(|source| ServerError::Render {
            source: Box::new(source),
        })?;
        Pages::from_rendered(output, previous.as_deref())
    };
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|source| ServerError::Worker { source })?
}
