//! The reader pass: a cookie, signed with the site's key, that a reader
//! gets when they confirm their address and that lets them read gated
//! content on every post. A subscriber on a new device asks again with the
//! same form and gets an email with a short-lived link that hands them a
//! new cookie. Neither holds the address, and neither can be made without
//! the key (task 5.11).

use std::fmt;
use std::sync::Arc;

use chrono::TimeDelta;
use hmac::{Hmac, KeyInit, Mac};
use khalid_core::{Clock, Secret};
use sha2::Sha256;

use crate::SiteLinks;

/// HMAC with SHA-256.
type HmacSha256 = Hmac<Sha256>;

/// How long a reader stays let in after confirming, or after opening a
/// pass link.
pub const PASS_DAYS: i64 = 180;

/// How long a pass link in an email works, the same as a confirmation link.
const LINK_DAYS: i64 = 2;

/// The cookie's name.
const COOKIE: &str = "khalid_reader";

/// What a signature is over, so a link code is never a cookie and back.
#[derive(Debug, Clone, Copy)]
enum Use {
    Cookie,
    Link,
}

/// Makes and checks reader cookies and pass links with the site's 256-bit
/// key. Cloning shares the clock.
#[derive(Clone)]
pub struct ReaderPass {
    key: HmacSha256,
    site: SiteLinks,
    clock: Arc<dyn Clock>,
}

impl fmt::Debug for ReaderPass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ReaderPass([redacted])")
    }
}

impl ReaderPass {
    /// A pass signed with `key` for the blog at `site`, timed by `clock`.
    /// The cookie is marked `Secure` when the site's address is `https`.
    pub fn new(key: &Secret<[u8; 32]>, site: &SiteLinks, clock: Arc<dyn Clock>) -> Self {
        // HMAC pads a short key with zeros, so padding it here gives the
        // same MAC with no length to check.
        let mut block = [0_u8; 64];
        block
            .iter_mut()
            .zip(key.expose())
            .for_each(|(to, from)| *to = *from);
        Self {
            key: HmacSha256::new(&block.into()),
            site: site.clone(),
            clock,
        }
    }

    /// The `Set-Cookie` value that lets a reader in for [`PASS_DAYS`]. It
    /// is sent to every page of the blog, is never readable by scripts,
    /// and is not sent on requests from other sites' forms.
    pub fn cookie(&self) -> String {
        let secure = if self.site.is_https() { "; Secure" } else { "" };
        format!(
            "{COOKIE}={}; Path={}; Max-Age={}; HttpOnly; SameSite=Lax{secure}",
            self.code(Use::Cookie, PASS_DAYS),
            self.site.mount(),
            TimeDelta::days(PASS_DAYS).num_seconds(),
        )
    }

    /// Whether `cookies`, a request's `Cookie` header, holds a reader
    /// cookie this site made, still in time. The signature is compared in
    /// constant time.
    pub fn admits(&self, cookies: &str) -> bool {
        let value = cookies.split(';').find_map(|pair| {
            let (key, value) = pair.trim().split_once('=')?;
            (key == COOKIE).then_some(value)
        });
        value.is_some_and(|value| self.checks(Use::Cookie, value))
    }

    /// The link, for an email, that gives a returning subscriber a new
    /// cookie and brings them to `back`, a path on the blog. It works for
    /// two days.
    pub fn link(&self, back: &str) -> String {
        self.site.pass(&self.code(Use::Link, LINK_DAYS), back)
    }

    /// Whether `code`, from a pass link, is one this site made, still in
    /// time.
    pub fn opens(&self, code: &str) -> bool {
        self.checks(Use::Link, code)
    }

    /// `until.signature`, good for `days` from now.
    fn code(&self, used: Use, days: i64) -> String {
        let until = self.clock.now().timestamp() + TimeDelta::days(days).num_seconds();
        let sig = self.mac(used, until).finalize().into_bytes();
        format!("{until}.{}", hex(&sig))
    }

    /// Whether `code` is `until.signature` for `used`, signed here and not
    /// yet run out.
    fn checks(&self, used: Use, code: &str) -> bool {
        let Some((until, sig)) = code.split_once('.') else {
            return false;
        };
        let (Ok(until), Some(sig)) = (until.parse::<i64>(), unhex(sig)) else {
            return false;
        };
        let signed = self.mac(used, until).verify_slice(&sig).is_ok();
        signed && self.clock.now().timestamp() < until
    }

    /// The MAC over what the code is for and when it ends.
    fn mac(&self, used: Use, until: i64) -> HmacSha256 {
        let label: &[u8] = match used {
            Use::Cookie => b"khalid-reader-cookie\n",
            Use::Link => b"khalid-reader-link\n",
        };
        let mut mac = self.key.clone();
        mac.update(label);
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
