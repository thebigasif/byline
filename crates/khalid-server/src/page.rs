//! One rendered file the server answers with.

use axum::http::HeaderValue;
use bytes::Bytes;

use crate::compress::Compressed;
use crate::{MediaType, PagePath, etag};

/// One rendered page or file, ready to send: its address, its kind, its
/// bytes and the tag made from them. [`Pages`](crate::Pages) adds its
/// Brotli copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    path: PagePath,
    media_type: MediaType,
    body: Bytes,
    etag: Option<HeaderValue>,
    brotli: Option<Compressed>,
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
            brotli: None,
        }
    }

    /// The page with its Brotli copy, when compressing makes it smaller.
    pub(crate) fn compressed(self) -> Self {
        let brotli = Compressed::brotli(&self.body, self.etag.as_ref());
        Self { brotli, ..self }
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

    /// The Brotli copy, if the page has one.
    pub(crate) fn brotli(&self) -> Option<&Compressed> {
        self.brotli.as_ref()
    }
}
