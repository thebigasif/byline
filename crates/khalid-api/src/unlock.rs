//! The cookie that opens a password-protected post for a reader who gave
//! its password: one cookie per post, signed with the site's key, so it
//! cannot be made without the key, moved to another post, or kept working
//! after the password changes (ADR "a password page is a plain form and a
//! signed cookie").

use std::fmt;
use std::sync::Arc;

use chrono::TimeDelta;
use hmac::{Hmac, KeyInit, Mac};
use khalid_core::{Clock, PasswordHash, Secret};
use sha2::{Digest, Sha256};

/// HMAC with SHA-256.
type HmacSha256 = Hmac<Sha256>;

/// How long a reader stays let in after giving the password.
pub const UNLOCK_DAYS: i64 = 30;

/// Makes and checks unlock cookies with the site's 256-bit key.
#[derive(Clone)]
pub struct UnlockSigner {
    key: HmacSha256,
    secure: bool,
    clock: Arc<dyn Clock>,
}

impl fmt::Debug for UnlockSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UnlockSigner([redacted])")
    }
}

/// Whether the site is served over HTTPS, so the cookie is sent only
/// there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// Plain HTTP, such as a local `khalid serve`.
    Http,
    /// HTTPS: the cookie gets `Secure`.
    Https,
}

impl UnlockSigner {
    /// A signer with the site's `key`, for a site served over `transport`,
    /// timing cookies by `clock`.
    pub fn new(key: &Secret<[u8; 32]>, transport: Transport, clock: impl Clock + 'static) -> Self {
        // HMAC pads a short key with zeros, so padding it here gives the
        // same MAC with no length to check.
        let mut block = [0_u8; 64];
        block
            .iter_mut()
            .zip(key.expose())
            .for_each(|(to, from)| *to = *from);
        Self {
            key: HmacSha256::new(&block.into()),
            secure: transport == Transport::Https,
            clock: Arc::new(clock),
        }
    }

    /// The `Set-Cookie` value that opens the post at `path`, whose password
    /// has `hash`, for [`UNLOCK_DAYS`]. It is sent only to that address, is
    /// never readable by scripts, and is not sent on requests from other
    /// sites' forms.
    pub fn cookie(&self, path: &str, hash: &PasswordHash) -> String {
        let until = self.clock.now().timestamp() + TimeDelta::days(UNLOCK_DAYS).num_seconds();
        let sig = hex(&self.mac(path, hash, until).finalize().into_bytes());
        let secure = if self.secure { "; Secure" } else { "" };
        format!(
            "{}={until}.{sig}; Path={path}; Max-Age={}; HttpOnly; SameSite=Lax{secure}",
            name(path),
            TimeDelta::days(UNLOCK_DAYS).num_seconds(),
        )
    }

    /// Whether `cookies`, a request's `Cookie` header, holds a cookie this
    /// site made for the post at `path` with its password's `hash`, still
    /// in time. The signature is compared in constant time.
    pub fn opens(&self, cookies: &str, path: &str, hash: &PasswordHash) -> bool {
        let wanted = name(path);
        let value = cookies.split(';').find_map(|pair| {
            let (key, value) = pair.trim().split_once('=')?;
            (key == wanted).then_some(value)
        });
        let Some((until, sig)) = value.and_then(|value| value.split_once('.')) else {
            return false;
        };
        let (Ok(until), Some(sig)) = (until.parse::<i64>(), unhex(sig)) else {
            return false;
        };
        let signed = self.mac(path, hash, until).verify_slice(&sig).is_ok();
        signed && self.clock.now().timestamp() < until
    }

    /// The MAC over the post's address, its password's hash and the end.
    fn mac(&self, path: &str, hash: &PasswordHash, until: i64) -> HmacSha256 {
        let mut mac = self.key.clone();
        mac.update(b"khalid-unlock\n");
        mac.update(path.as_bytes());
        mac.update(b"\n");
        mac.update(hash.as_str().as_bytes());
        mac.update(b"\n");
        mac.update(until.to_string().as_bytes());
        mac
    }
}

/// The cookie's name for the post at `path`: one per post, so opening one
/// post leaves the others locked.
fn name(path: &str) -> String {
    let digest = Sha256::digest(path.as_bytes());
    format!("khalid_unlock_{}", hex(digest.get(..8).unwrap_or_default()))
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
