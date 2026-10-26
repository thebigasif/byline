//! Entity tags made from a page's bytes, and the check a browser makes
//! when it asks whether its copy is still current.

use std::fmt::Write;

use axum::http::{HeaderMap, HeaderValue, header};
use sha2::{Digest, Sha256};

/// How many bytes of the SHA-256 digest a tag keeps. 128 bits is far more
/// than enough to tell two versions of one page apart.
const TAG_BYTES: usize = 16;

/// The strong tag for `body`: 32 hex digits in quotes. The
/// same bytes always give the same tag, on every start, so a browser's
/// copy stays valid across restarts.
///
/// A tag of hex digits is always a valid header value; `None` cannot
/// happen, and would only mean the page is sent without a tag.
pub(crate) fn of(body: &[u8]) -> Option<HeaderValue> {
    let digest = Sha256::digest(body);
    let mut tag = String::with_capacity(TAG_BYTES * 2 + 2);
    tag.push('"');
    digest
        .iter()
        .take(TAG_BYTES)
        .try_for_each(|byte| write!(tag, "{byte:02x}"))
        .ok()?;
    tag.push('"');
    HeaderValue::try_from(tag).ok()
}

/// Whether the request's `If-None-Match` names `tag` or `*`. The
/// comparison is weak, as RFC 9110 section 13.1.2 asks: `W/"x"` matches
/// `"x"`.
pub(crate) fn is_current(headers: &HeaderMap, tag: &HeaderValue) -> bool {
    headers
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .any(|sent| sent == "*" || sent.strip_prefix("W/").unwrap_or(sent) == tag)
}
