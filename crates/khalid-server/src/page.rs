//! One rendered file the server answers with.

use bytes::Bytes;

use crate::{MediaType, PagePath};

/// One rendered page or file, ready to send: its address, its kind and its
/// bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    path: PagePath,
    media_type: MediaType,
    body: Bytes,
}

impl Page {
    /// A page at `path` holding `body`.
    pub fn new(path: PagePath, media_type: MediaType, body: impl Into<Bytes>) -> Self {
        Self {
            path,
            media_type,
            body: body.into(),
        }
    }

    /// Where the page lives on the site.
    pub fn path(&self) -> &PagePath {
        &self.path
    }

    /// What kind of file it is.
    pub fn media_type(&self) -> MediaType {
        self.media_type
    }

    /// The bytes sent to a reader.
    pub fn body(&self) -> &Bytes {
        &self.body
    }
}
