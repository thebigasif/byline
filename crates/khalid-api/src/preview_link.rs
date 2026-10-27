//! Preview links: an address that shows a draft, in the site's own
//! design, to someone without an account, until it expires (CTO answers
//! 2026-10-27, section 4). Each is signed with the site's preview key, so
//! nobody can make one or stretch its time without the key.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use hmac::{Hmac, KeyInit, Mac};
use khalid_core::{AbsoluteUrl, Clock, Secret};
use serde::Serialize;
use sha2::Sha256;
use utoipa::ToSchema;

/// HMAC with SHA-256.
type HmacSha256 = Hmac<Sha256>;

/// The folder preview links live in, under the blog's own folder.
pub const PREVIEW_FOLDER: &str = "preview";

/// A link to a draft, and when it stops working.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct PreviewLink {
    /// The address to share, such as
    /// `https://www.example.com/blog/preview/en.choosing-anchor-text/?until=1793696400&sig=...`.
    #[serde(serialize_with = "crate::resources::display")]
    #[schema(value_type = String)]
    url: AbsoluteUrl,
    /// When the link stops working.
    expires_at: DateTime<Utc>,
}

impl PreviewLink {
    /// The address to share.
    pub fn url(&self) -> &AbsoluteUrl {
        &self.url
    }

    /// When the link stops working.
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
}

/// Why a preview link does not open its draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PreviewError {
    /// The link's time has passed.
    #[error("the preview link has expired")]
    Expired,
    /// The signature does not match the post and time: the link was
    /// changed, or made with another key.
    #[error("the preview link is not one this site made")]
    Forged,
}

/// Makes and checks a site's preview links. `Debug` never shows the key.
#[derive(Clone)]
pub struct PreviewSigner {
    key: HmacSha256,
    blog: String,
    lifetime: TimeDelta,
    clock: Arc<dyn Clock>,
}

impl fmt::Debug for PreviewSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreviewSigner")
            .field("blog", &self.blog)
            .field("lifetime", &self.lifetime)
            .finish_non_exhaustive()
    }
}

impl PreviewSigner {
    /// A signer with the site's 256-bit preview `key`, for the blog at
    /// `blog`, its address with the mount folder, making links that work
    /// for `lifetime` from `clock`'s now.
    pub fn new(
        key: &Secret<[u8; 32]>,
        blog: &AbsoluteUrl,
        lifetime: Duration,
        clock: impl Clock + 'static,
    ) -> Self {
        // HMAC pads a key shorter than SHA-256's 64-byte block with zeros,
        // so padding it here gives the same MAC with no length to check.
        let mut block = [0_u8; 64];
        block
            .iter_mut()
            .zip(key.expose())
            .for_each(|(to, from)| *to = *from);
        let key = HmacSha256::new(&block.into());
        let blog = format!("{}/", blog.as_str().trim_end_matches('/'));
        Self {
            key,
            blog,
            lifetime: TimeDelta::from_std(lifetime).unwrap_or(TimeDelta::MAX),
            clock: Arc::new(clock),
        }
    }

    /// A new link to post `id`, working until the lifetime has passed, or
    /// `None` when `id` cannot be part of an address.
    pub fn link(&self, id: &str) -> Option<PreviewLink> {
        let expires_at = self
            .clock
            .now()
            .checked_add_signed(self.lifetime)
            .unwrap_or(DateTime::<Utc>::MAX_UTC);
        let until = expires_at.timestamp();
        let sig = hex(&self.mac(id, until).finalize().into_bytes());
        let url = format!(
            "{}{PREVIEW_FOLDER}/{id}/?until={until}&sig={sig}",
            self.blog
        );
        let url = AbsoluteUrl::parse(&url).ok()?;
        Some(PreviewLink { url, expires_at })
    }

    /// Whether a link to post `id` with `until` and `sig` was made by this
    /// site and still works. The signature is compared in constant time.
    ///
    /// # Errors
    ///
    /// Returns [`PreviewError::Forged`] when `sig` does not match `id` and
    /// `until`, and [`PreviewError::Expired`] when it does and `until` has
    /// passed.
    pub fn check(&self, id: &str, until: i64, sig: &str) -> Result<(), PreviewError> {
        let sig = unhex(sig).ok_or(PreviewError::Forged)?;
        self.mac(id, until)
            .verify_slice(&sig)
            .map_err(|_| PreviewError::Forged)?;
        if self.clock.now().timestamp() >= until {
            return Err(PreviewError::Expired);
        }
        Ok(())
    }

    /// The MAC over the post's identity and the link's end.
    fn mac(&self, id: &str, until: i64) -> HmacSha256 {
        let mut mac = self.key.clone();
        mac.update(b"khalid-preview\n");
        mac.update(id.as_bytes());
        mac.update(b"\n");
        mac.update(until.to_string().as_bytes());
        mac
    }
}

/// `bytes` as lowercase hex digits.
fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .filter_map(|nibble| char::from_digit(u32::from(nibble), 16))
        .collect()
}

/// Hex digits as bytes, or `None`.
fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    text.as_bytes()
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}
