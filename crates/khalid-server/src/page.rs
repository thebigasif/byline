//! One rendered file the server answers with.

use axum::http::HeaderValue;
use bytes::Bytes;

use crate::compress::Compressed;
use crate::{MediaType, PagePath, etag};

/// Pages may be kept, but must be checked with their tag before reuse, so
/// an edited post shows on the next visit.
const REVALIDATE: HeaderValue = HeaderValue::from_static("no-cache");

/// A file whose name changes with its content may be kept for a year, the
/// most RFC 9111 advises, and is never checked again.
const IMMUTABLE: HeaderValue = HeaderValue::from_static("public, max-age=31536000, immutable");

/// How long a reader may keep a page before asking for it again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Caching {
    /// Kept, but checked with its tag before each reuse.
    Revalidate,
    /// Kept for a year without a check, because its address names its content.
    Immutable,
}

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
    caching: Caching,
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
            caching: Caching::Revalidate,
        }
    }

    /// A file at `path` whose name carries a hash of `body`, such as
    /// `/blog/assets/site.3f2a9c1e.css`. Readers keep it for a year without
    /// asking again, because a change to the file gives it a new name.
    pub fn immutable(path: PagePath, media_type: MediaType, body: impl Into<Bytes>) -> Self {
        Self {
            caching: Caching::Immutable,
            ..Self::new(path, media_type, body)
        }
    }

    /// The page with its Brotli copy, when its kind is text and compressing
    /// makes it smaller. When `old` has the same bytes, by its tag, and a
    /// copy, that copy is shared instead of compressing again.
    pub(crate) fn compressed_like(self, old: Option<&Self>) -> Self {
        let brotli = match old {
            _ if !self.media_type.compresses() => None,
            Some(old) if old.etag.is_some() && old.etag == self.etag && old.brotli.is_some() => {
                old.brotli.clone()
            }
            _ => Compressed::brotli(&self.body, self.etag.as_ref()),
        };
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

    /// The `Cache-Control` header value.
    pub(crate) fn cache_control(&self) -> HeaderValue {
        match self.caching {
            Caching::Revalidate => REVALIDATE,
            Caching::Immutable => IMMUTABLE,
        }
    }
}
