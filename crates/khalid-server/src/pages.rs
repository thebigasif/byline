//! Every page of the site, held in memory.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use bytes::Bytes;

use crate::{Page, Rendered, ServerError};

/// Every rendered page of one site by its path, and the page for an address
/// with nothing behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pages {
    by_path: BTreeMap<String, Page>,
    not_found: Bytes,
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

    /// Holds the `rendered` pages. A page whose bytes are the same as the
    /// page at its address in `previous` takes that page's Brotli copy
    /// instead of compressing again, so a change to one post costs one
    /// compression.
    pub(crate) fn from_rendered(
        rendered: Rendered,
        previous: Option<&Self>,
    ) -> Result<Self, ServerError> {
        let (pages, not_found) = rendered.into_parts();
        let mut by_path = BTreeMap::new();
        for page in pages {
            let path = page.path().as_str().to_owned();
            let old = previous.and_then(|pages| pages.get(&path));
            match by_path.entry(path) {
                Entry::Occupied(_) => {
                    return Err(ServerError::DuplicatePage {
                        path: page.path().clone(),
                    });
                }
                Entry::Vacant(slot) => {
                    slot.insert(page.compressed_like(old));
                }
            }
        }
        Ok(Self { by_path, not_found })
    }

    /// The page at `path`, once percent-encoding is undone.
    pub fn get(&self, path: &str) -> Option<&Page> {
        self.by_path.get(path)
    }

    /// The HTML for an address with nothing behind it.
    pub fn not_found(&self) -> &Bytes {
        &self.not_found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MediaType, PagePath};

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
}
