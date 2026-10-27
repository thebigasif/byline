//! Every page of the site, held in memory.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use bytes::Bytes;

use crate::{Moved, Page, PagePath, Rendered, ServerError};

/// Every rendered page of one site by its path, the page for an address
/// with nothing behind it, and where each moved address went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pages {
    by_path: BTreeMap<String, Page>,
    not_found: Bytes,
    moved: BTreeMap<String, PagePath>,
}

impl Pages {
    /// Holds `pages`, with `not_found` as the HTML for any other address,
    /// and compresses each page with Brotli once, here.
    ///
    /// Compressing is CPU work, so call this off the async runtime, for
    /// example in `spawn_blocking` (RULES.md BL-RUN-7).
    ///
    /// # Errors
    ///
    /// Returns [`ServerError::DuplicatePage`] when two pages share a path,
    /// because only one of them could ever be read.
    pub fn new(pages: Vec<Page>, not_found: impl Into<Bytes>) -> Result<Self, ServerError> {
        Self::from_rendered(Rendered::new(pages, not_found), None)
    }

    /// Holds `pages` as they are, without Brotli copies, so a server can
    /// answer at once and add them later with [`Pages::compressed`]. On a
    /// 1,000-post site, compressing first would hold the first answer back
    /// by most of a second.
    ///
    /// # Errors
    ///
    /// Returns [`ServerError::DuplicatePage`] when two pages share a path.
    pub fn plain(pages: Vec<Page>, not_found: impl Into<Bytes>) -> Result<Self, ServerError> {
        let mut by_path = BTreeMap::new();
        for page in pages {
            match by_path.entry(page.path().as_str().to_owned()) {
                Entry::Occupied(_) => {
                    return Err(ServerError::DuplicatePage {
                        path: page.path().clone(),
                    });
                }
                Entry::Vacant(slot) => {
                    slot.insert(page);
                }
            }
        }
        Ok(Self {
            by_path,
            not_found: not_found.into(),
            moved: BTreeMap::new(),
        })
    }

    /// The same pages, with each of `moves` answered by 301 where no page
    /// sits. A later move from the same address wins.
    #[must_use]
    pub fn with_moves(mut self, moves: Vec<Moved>) -> Self {
        self.moved = moves
            .into_iter()
            .map(|moved| (moved.from().as_str().to_owned(), moved.to().clone()))
            .collect();
        self
    }

    /// The same pages, each with its Brotli copy. A page that has one
    /// already keeps it. The bytes are shared, not copied.
    ///
    /// Compressing is CPU work, so call this off the async runtime
    /// (RULES.md BL-RUN-7).
    #[must_use]
    pub fn compressed(&self) -> Self {
        let by_path = self
            .by_path
            .iter()
            .map(|(path, page)| (path.clone(), page.clone().compressed_like(Some(page))))
            .collect();
        Self {
            by_path,
            not_found: self.not_found.clone(),
            moved: self.moved.clone(),
        }
    }

    /// Holds the `rendered` pages. A page whose bytes are the same as the
    /// page at its address in `previous` takes that page's Brotli copy
    /// instead of compressing again, so a change to one post costs one
    /// compression.
    pub(crate) fn from_rendered(
        rendered: Rendered,
        previous: Option<&Self>,
    ) -> Result<Self, ServerError> {
        let (pages, not_found, moves) = rendered.into_parts();
        let mut plain = Self::plain(pages, not_found)?.with_moves(moves);
        for (path, page) in &mut plain.by_path {
            let old = previous.and_then(|pages| pages.get(path));
            *page = page.clone().compressed_like(old);
        }
        Ok(plain)
    }

    /// The page at `path`, once percent-encoding is undone.
    pub fn get(&self, path: &str) -> Option<&Page> {
        self.by_path.get(path)
    }

    /// The HTML for an address with nothing behind it.
    pub fn not_found(&self) -> &Bytes {
        &self.not_found
    }

    /// Where the address `path` moved to, once percent-encoding is undone.
    pub fn moved(&self, path: &str) -> Option<&PagePath> {
        self.moved.get(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MediaType, Moved, PagePath};

    fn post(body: &str) -> Page {
        let path = PagePath::try_new("/blog/what-a-sitemap-does/").unwrap();
        Page::new(path, MediaType::Html, body.repeat(40))
    }

    fn brotli_of(pages: &Pages) -> *const u8 {
        let page = pages.get("/blog/what-a-sitemap-does/").unwrap();
        page.brotli().unwrap().body().as_ptr()
    }

    #[test]
    fn an_unchanged_page_keeps_its_brotli_copy() {
        let first = Pages::new(vec![post("<p>A sitemap lists pages.</p>")], "").unwrap();
        let same = Rendered::new(vec![post("<p>A sitemap lists pages.</p>")], "");
        let edited = Rendered::new(vec![post("<p>A sitemap lists posts.</p>")], "");
        let again = Pages::from_rendered(same, Some(&first)).unwrap();
        let changed = Pages::from_rendered(edited, Some(&first)).unwrap();
        assert_eq!(
            brotli_of(&again),
            brotli_of(&first),
            "reused, not compressed again"
        );
        assert_ne!(brotli_of(&changed), brotli_of(&first));
    }

    #[test]
    fn rendered_moves_reach_the_pages() {
        let path = |text: &str| PagePath::try_new(text).unwrap();
        let moved = Moved::new(path("/blog/old/"), path("/blog/what-a-sitemap-does/"));
        let rendered = Rendered::new(vec![post("<p>A sitemap.</p>")], "").with_moves(vec![moved]);
        let pages = Pages::from_rendered(rendered, None).unwrap();
        let to = pages.moved("/blog/old/").map(PagePath::as_str);
        assert_eq!(to, Some("/blog/what-a-sitemap-does/"));
        assert_eq!(
            pages.compressed().moved("/blog/old/"),
            pages.moved("/blog/old/")
        );
    }
}
