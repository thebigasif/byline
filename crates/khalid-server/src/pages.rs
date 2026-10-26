//! Every page of the site, held in memory.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use bytes::Bytes;

use crate::{Page, ServerError};

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
        let mut by_path = BTreeMap::new();
        for page in pages {
            match by_path.entry(page.path().as_str().to_owned()) {
                Entry::Occupied(_) => {
                    return Err(ServerError::DuplicatePage {
                        path: page.path().clone(),
                    });
                }
                Entry::Vacant(slot) => {
                    slot.insert(page.compressed());
                }
            }
        }
        Ok(Self {
            by_path,
            not_found: not_found.into(),
        })
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
