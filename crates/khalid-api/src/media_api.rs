//! The media library under `/api/admin/v1/media/` (plan section 5.15).

use std::sync::Arc;

use khalid_media::{Library, MediaItem};

use crate::resources::AdminMedia;

/// A site's media library, and the address its own files are served under.
#[derive(Debug, Clone)]
pub struct MediaLibrary {
    library: Arc<Library>,
    site: String,
}

impl MediaLibrary {
    /// `library`, whose files on the site itself are served under `site`,
    /// the blog's address with its mount, such as
    /// `https://www.example.com/blog/`.
    pub fn new(library: Arc<Library>, site: &str) -> Self {
        let site = format!("{}/", site.trim_end_matches('/'));
        Self { library, site }
    }

    /// `item` as the Admin API sends it, with the full address readers
    /// fetch its file from.
    pub fn shown(&self, item: &MediaItem) -> AdminMedia {
        AdminMedia::new(item, self.library.address(item), &self.site)
    }
}
