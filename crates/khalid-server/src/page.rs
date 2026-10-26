//! One rendered file the server answers with.

use axum::http::HeaderValue;
use bytes::Bytes;

use crate::{MediaType, PagePath, etag};

/// One rendered page or file, ready to send: its address, its kind, its
/// bytes and the tag made from them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    path: PagePath,
    media_type: MediaType,
    body: Bytes,
    etag: Option<HeaderValue>,
}

impl Page {
    /// A page at `path` holding `body`. Its entity tag is worked out here,
    /// once, so a request only compares it.
    pub fn new(path: PagePath, media_type: MediaType, body: impl Into<Bytes>) -> Self {
        let body = body.into();
        Self {
            etag: etag::of(&body),
            path,
            media_type,
            body,
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

    /// The `ETag` header value, made from the bytes.
    pub(crate) fn etag(&self) -> Option<&HeaderValue> {
        self.etag.as_ref()
    }
}
