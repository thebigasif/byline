//! The text of a key: `kc_<id>_<secret>`.

use std::str::FromStr;

use khalid_core::Secret;

/// Hex digits in a key's identity: 64 bits, enough to look a key up.
const ID_DIGITS: usize = 16;
/// Hex digits in a key's secret: 256 bits (RULES.md BL-SEC-3).
const SECRET_DIGITS: usize = 64;

/// What a key is for, read from its prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyKind {
    /// `kc_`: reads published content. Safe to put in a web page.
    Content,
}

impl KeyKind {
    fn from_prefix(prefix: &str) -> Option<Self> {
        match prefix {
            "kc" => Some(Self::Content),
            _ => None,
        }
    }
}

/// A key's public identity: 16 lowercase hex digits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyId(String);

impl KeyId {
    /// Checks `text` is 16 lowercase hex digits.
    ///
    /// # Errors
    ///
    /// Returns [`ParseKeyError`] when it is not.
    pub fn try_new(text: &str) -> Result<Self, ParseKeyError> {
        is_hex(text, ID_DIGITS)
            .then(|| Self(text.to_owned()))
            .ok_or(ParseKeyError)
    }

    /// The identity as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A whole key as a client sends it: `kc_<16 hex digits>_<64 hex digits>`.
/// The secret stays wrapped, so `Debug` never shows it (BL-RUN-5).
#[derive(Debug)]
pub struct KeyToken {
    kind: KeyKind,
    id: KeyId,
    secret: Secret<String>,
}

impl KeyToken {
    /// What the key is for.
    pub fn kind(&self) -> KeyKind {
        self.kind
    }

    /// Its public identity.
    pub fn id(&self) -> &KeyId {
        &self.id
    }

    pub(crate) fn secret(&self) -> &Secret<String> {
        &self.secret
    }
}

impl FromStr for KeyToken {
    type Err = ParseKeyError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parts = text.split('_');
        let (Some(prefix), Some(id), Some(secret), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(ParseKeyError);
        };
        let kind = KeyKind::from_prefix(prefix).ok_or(ParseKeyError)?;
        let id = KeyId::try_new(id)?;
        if !is_hex(secret, SECRET_DIGITS) {
            return Err(ParseKeyError);
        }
        Ok(Self {
            kind,
            id,
            secret: Secret::new(secret.to_owned()),
        })
    }
}

/// Whether `text` is exactly `digits` lowercase hex digits.
fn is_hex(text: &str, digits: usize) -> bool {
    text.len() == digits
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Text that is not a key. It says nothing about which part was wrong, so
/// an error never helps a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("this is not a Khalid key")]
pub struct ParseKeyError;
