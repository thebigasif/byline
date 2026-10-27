//! One rendered file the server answers with.

use axum::http::HeaderValue;
use bytes::Bytes;

use crate::compress::Compressed;
use crate::{MediaType, PagePath, ServerError, etag};

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
    /// Never kept, because each answer fills [`NONCE_SLOT`] with that
    /// request's nonce, so no two answers are the same bytes.
    PerRequest,
}

/// What a page with a post's head code holds where each answer puts its
/// request's nonce, such as `<script nonce="khalid-nonce-slot">`. Only hex
/// digits replace it, so it can never open a tag.
pub const NONCE_SLOT: &str = "khalid-nonce-slot";

/// A page that must not be kept, because its bytes differ on each request.
const NO_STORE: HeaderValue = HeaderValue::from_static("no-store");

/// One rendered page or file, ready to send: its address, its kind, its
/// bytes and the tag made from them. [`Pages`](crate::Pages) adds its
/// Brotli copy, and then keeps that copy alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    path: PagePath,
    media_type: MediaType,
    stored: Stored,
    /// The length of the plain bytes.
    len: usize,
    /// The tag of the plain bytes.
    etag: Option<HeaderValue>,
    caching: Caching,
}

/// How a page's bytes are held: one copy, never two.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Stored {
    /// The plain bytes, before compressing, or when it would not shrink
    /// them.
    Plain(Bytes),
    /// The Brotli copy only. Most readers take it as it is; for the rest it
    /// is decoded when they ask.
    Brotli(Compressed),
}

impl Page {
    /// A page at `path` holding `body`. Its entity tag is worked out here,
    /// once, so a request only compares it.
    pub fn new(path: PagePath, media_type: MediaType, body: impl Into<Bytes>) -> Self {
        let body = body.into();
        Self {
            etag: etag::of(&body),
            len: body.len(),
            path,
            media_type,
            stored: Stored::Plain(body),
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

    /// A page at `path` whose `body` holds [`NONCE_SLOT`] wherever a post's
    /// head code needs the request's nonce. Each answer fills the slot and
    /// carries a policy with that nonce, so the page has no tag and no
    /// Brotli copy, and is never kept (ADR "head code gets the nonce on
    /// each request").
    pub fn with_nonce_slot(path: PagePath, media_type: MediaType, body: impl Into<Bytes>) -> Self {
        Self {
            caching: Caching::PerRequest,
            etag: None,
            ..Self::new(path, media_type, body)
        }
    }

    /// Whether each answer must fill the page's [`NONCE_SLOT`].
    pub(crate) fn fills_nonce(&self) -> bool {
        self.caching == Caching::PerRequest
    }

    /// The page holding only its Brotli copy, when its kind is text and
    /// compressing makes it smaller, or as it is. When `old` has the same
    /// bytes, by its tag, and a copy, that copy is shared instead of
    /// compressing again.
    pub(crate) fn compressed_like(self, old: Option<&Self>) -> Self {
        let Stored::Plain(plain) = &self.stored else {
            return self;
        };
        let brotli = match old {
            _ if !self.media_type.compresses() || self.fills_nonce() => None,
            Some(old) if old.etag.is_some() && old.etag == self.etag && old.brotli().is_some() => {
                old.brotli().cloned()
            }
            _ => Compressed::brotli(plain, self.etag.as_ref()),
        };
        match brotli {
            Some(copy) => Self {
                stored: Stored::Brotli(copy),
                ..self
            },
            None => self,
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

    /// The plain bytes sent to a reader that does not take Brotli. When the
    /// page holds only its Brotli copy, they are decoded from it now.
    ///
    /// # Errors
    ///
    /// Returns [`ServerError::Decompress`] when the Brotli copy cannot be
    /// decoded, which only a fault in this crate could cause.
    pub fn body(&self) -> Result<Bytes, ServerError> {
        match &self.stored {
            Stored::Plain(plain) => Ok(plain.clone()),
            Stored::Brotli(copy) => {
                copy.plain(self.len)
                    .map_err(|source| ServerError::Decompress {
                        path: self.path.clone(),
                        source,
                    })
            }
        }
    }

    /// The length of the plain bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the page has no bytes at all.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The `ETag` header value, made from the bytes.
    pub(crate) fn etag(&self) -> Option<&HeaderValue> {
        self.etag.as_ref()
    }

    /// The Brotli copy, if the page has one.
    pub(crate) fn brotli(&self) -> Option<&Compressed> {
        match &self.stored {
            Stored::Brotli(copy) => Some(copy),
            Stored::Plain(_) => None,
        }
    }

    /// The `Cache-Control` header value.
    pub(crate) fn cache_control(&self) -> HeaderValue {
        match self.caching {
            Caching::Revalidate => REVALIDATE,
            Caching::Immutable => IMMUTABLE,
            Caching::PerRequest => NO_STORE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post(body: &str) -> Page {
        let path = PagePath::try_new("/blog/what-a-sitemap-does/").unwrap();
        Page::new(path, MediaType::Html, body.to_owned())
    }

    #[test]
    fn a_compressed_page_keeps_only_its_brotli_copy_and_gives_its_bytes_back() {
        let text = "<p>A sitemap lists the pages you want found.</p>\n".repeat(40);
        let page = post(&text).compressed_like(None);
        assert!(
            matches!(page.stored, Stored::Brotli(_)),
            "{:?}",
            page.stored
        );
        assert_eq!(page.len(), text.len());
        assert_eq!(page.body().unwrap(), text.as_bytes());
    }

    #[test]
    fn a_page_compressing_cannot_shrink_stays_plain() {
        let page = post("<p>Hi</p>").compressed_like(None);
        assert!(matches!(page.stored, Stored::Plain(_)), "{:?}", page.stored);
        assert!(page.brotli().is_none());
        assert_eq!(page.body().unwrap(), "<p>Hi</p>".as_bytes());
    }
}
