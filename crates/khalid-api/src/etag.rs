//! `ETag` and cache headers on successful reads.

use std::fmt::Write as _;

use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::http::header::{CACHE_CONTROL, ETAG, IF_NONE_MATCH, VARY};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};

use crate::problem::Problem;

/// Caches may keep a copy but must ask again each time, which costs one
/// small 304 when nothing changed.
const CACHE: &str = "public, max-age=0, must-revalidate";
/// The request headers that change the answer.
const VARIES: &str = "authorization, accept-version";
/// Bytes of the body's SHA-256 used in the tag: 128 bits.
const TAG_BYTES: usize = 16;

/// Tags every 200 answer with a strong `ETag` from its body, with cache
/// headers, and answers 304 with no body when `If-None-Match` already
/// holds that tag.
pub(crate) async fn tag_and_revalidate(request: Request, next: Next) -> Response {
    let asked = request.headers().get(IF_NONE_MATCH).cloned();
    let response = next.run(request).await;
    if response.status() != StatusCode::OK {
        return response;
    }
    let (mut parts, body) = response.into_parts();
    let bytes = match to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(error) => return Problem::internal(&error).into_response(),
    };
    let tag = tag(&bytes);
    let fresh = asked
        .as_ref()
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| holds(value, &tag));
    let tag = HeaderValue::from_str(&tag).unwrap_or_else(|_| HeaderValue::from_static("\"\""));
    add_headers(&mut parts.headers, tag);
    if fresh {
        let mut not_modified = StatusCode::NOT_MODIFIED.into_response();
        for name in [ETAG, CACHE_CONTROL, VARY] {
            if let Some(value) = parts.headers.get(&name) {
                not_modified.headers_mut().insert(name, value.clone());
            }
        }
        return not_modified;
    }
    Response::from_parts(parts, Body::from(bytes))
}

fn add_headers(headers: &mut HeaderMap, tag: HeaderValue) {
    headers.insert(ETAG, tag);
    headers.insert(CACHE_CONTROL, HeaderValue::from_static(CACHE));
    headers.insert(VARY, HeaderValue::from_static(VARIES));
}

/// `"<32 hex digits>"` from the SHA-256 of `body`.
fn tag(body: &[u8]) -> String {
    let digest = Sha256::digest(body);
    let mut tag = String::from("\"");
    for byte in digest.iter().take(TAG_BYTES) {
        let _ = write!(tag, "{byte:02x}");
    }
    tag.push('"');
    tag
}

/// Whether an `If-None-Match` value names `tag`: `*`, or a list in which
/// one entry matches, weak or strong.
fn holds(value: &str, tag: &str) -> bool {
    value.trim() == "*"
        || value
            .split(',')
            .map(|entry| entry.trim().trim_start_matches("W/"))
            .any(|entry| entry == tag)
}
