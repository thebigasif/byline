//! Brotli copies of pages, made once when the pages are built, and whether
//! a browser accepts them.

use axum::http::{HeaderMap, HeaderValue, header};
use brotli::enc::BrotliEncoderParams;
use bytes::Bytes;

use crate::etag;

/// Brotli quality, from 0 to 11. On a 30 KB post, quality 5 takes about
/// 0.8 ms and quality 9 about 3.4 ms, for a page within a few percent of
/// the same size, so a rebuild of many pages stays quick.
const QUALITY: i32 = 5;

/// The smallest and largest Brotli window Brotli allows, as powers of two:
/// 1 KB and 4 MB.
const WINDOW: (i32, i32) = (10, 22);

/// The smallest Brotli window that holds all of a `len`-byte page, so the
/// encoder sets up buffers the size of the page, not 4 MB for each one.
/// A window that covers the whole page compresses it exactly as well.
pub(crate) fn window(len: usize) -> i32 {
    let bits = len.max(1).next_power_of_two().trailing_zeros();
    i32::try_from(bits).map_or(WINDOW.1, |bits| bits.clamp(WINDOW.0, WINDOW.1))
}

/// A page's bytes compressed with Brotli, and the tag for that copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Compressed {
    body: Bytes,
    etag: Option<HeaderValue>,
}

impl Compressed {
    /// The Brotli copy of `body`, whose plain copy is tagged `plain_tag`,
    /// or `None` when compressing would not make it smaller.
    ///
    /// This is CPU work: build pages off the async runtime (RULES.md
    /// BL-RUN-7).
    pub(crate) fn brotli(body: &[u8], plain_tag: Option<&HeaderValue>) -> Option<Self> {
        let params = BrotliEncoderParams {
            quality: QUALITY,
            lgwin: window(body.len()),
            size_hint: body.len(),
            ..BrotliEncoderParams::default()
        };
        let mut input = body;
        let mut output = Vec::with_capacity(body.len() / 4);
        brotli::BrotliCompress(&mut input, &mut output, &params).ok()?;
        if output.len() >= body.len() {
            return None;
        }
        Some(Self {
            body: Bytes::from(output),
            etag: plain_tag.and_then(|tag| etag::with_suffix(tag, "-br")),
        })
    }

    /// The compressed bytes.
    pub(crate) fn body(&self) -> &Bytes {
        &self.body
    }

    /// The tag of the compressed copy.
    pub(crate) fn etag(&self) -> Option<&HeaderValue> {
        self.etag.as_ref()
    }
}

/// Whether the request's `Accept-Encoding` takes Brotli, by name or by
/// `*`, without `q=0` refusing it.
pub(crate) fn accepts_brotli(headers: &HeaderMap) -> bool {
    let mut brotli = None;
    let mut any = None;
    let codings = headers
        .get_all(header::ACCEPT_ENCODING)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','));
    for coding in codings {
        let mut parts = coding.split(';');
        let name = parts.next().unwrap_or_default().trim();
        let accepted = !parts.any(|param| is_zero_weight(param.trim()));
        if name.eq_ignore_ascii_case("br") {
            brotli = Some(accepted);
        } else if name == "*" {
            any = Some(accepted);
        }
    }
    brotli.or(any).unwrap_or(false)
}

/// Whether a parameter is a weight of zero, such as `q=0` or `q=0.000`.
fn is_zero_weight(param: &str) -> bool {
    let weight = param
        .strip_prefix("q=")
        .or_else(|| param.strip_prefix("Q="));
    weight.is_some_and(|w| w.starts_with('0') && w.chars().all(|c| matches!(c, '0' | '.')))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_holds_the_whole_page_and_no_more() {
        assert_eq!(window(0), 10, "never below 1 KB");
        assert_eq!(window(700), 10);
        assert_eq!(window(1024), 10);
        assert_eq!(window(1025), 11);
        assert_eq!(window(7_000), 13);
        assert_eq!(window(30_000), 15);
        assert_eq!(window(50_000_000), 22, "never above 4 MB");
    }
}
