//! The name every cached card shares: a hash of all of its content.

use std::fmt;
use std::fmt::Write as _;

use sha2::{Digest, Sha256};

use crate::{Direction, ShareCard};

/// How many hex digits of SHA-256 name a card, as they do an original
/// image in `khalid-media`. Sixteen digits make a collision between two
/// cards of one site unthinkable while keeping file names short.
const KEY_DIGITS: usize = 16;

/// The layout this key hashes. Change it when the drawing changes, so no
/// card drawn by the old layout is served for the new one (ADR 0016).
const LAYOUT: &str = "khalid-share-image layout 1";

/// A hash of everything a card shows. Equal cards share one cached file, and
/// a changed title, site name, colour or direction gets a new key, so a
/// stale image is never served for a changed card.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShareKey(String);

impl ShareKey {
    /// The key of `card`.
    pub fn of(card: &ShareCard) -> Self {
        let mut hash = Sha256::new();
        feed(&mut hash, LAYOUT.as_bytes());
        feed(&mut hash, card.title.as_bytes());
        feed(&mut hash, card.site_name.as_bytes());
        feed(&mut hash, &card.colors.background.0);
        feed(&mut hash, &card.colors.text.0);
        feed(&mut hash, &card.colors.accent.0);
        let direction = match card.direction {
            Direction::LeftToRight => 0u8,
            Direction::RightToLeft => 1u8,
        };
        feed(&mut hash, &[direction]);
        let hash = hash.finalize();
        let mut key = String::with_capacity(KEY_DIGITS);
        for byte in hash.iter().take(KEY_DIGITS / 2) {
            let _ = write!(key, "{byte:02x}");
        }
        Self(key)
    }

    /// Reads a key written by [`ShareKey::as_str`], or `None` unless it is
    /// 16 lowercase hex digits, so a key can always be used as a file name.
    pub fn parse(text: &str) -> Option<Self> {
        let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        (text.len() == KEY_DIGITS && text.chars().all(hex)).then(|| Self(text.to_owned()))
    }

    /// The key as lowercase hex, such as `6ff9ee42a1733c1d`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ShareKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Feeds `field` to `hash` with its length first, so one field cannot be
/// read as two.
fn feed(hash: &mut Sha256, field: &[u8]) {
    let len = u64::try_from(field.len()).unwrap_or(u64::MAX);
    hash.update(len.to_le_bytes());
    hash.update(field);
}
