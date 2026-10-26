//! The text of a key: `kc_<id>_<secret>`.

use std::str::FromStr;

use khalid_core::Secret;

/// Hex digits in a key's identity: 64 bits, enough to look a key up.
const ID_DIGITS: usize = 16;
/// Hex digits in a key's secret: 256 bits (RULES.md BL-SEC-3).
const SECRET_DIGITS: usize = 64;
/// Random bytes in a new key's identity: two hex digits each.
const ID_BYTES: usize = 8;
/// Random bytes a new key is made from: 8 for its identity, then 32 for
/// its secret.
pub const RANDOM_BYTES: usize = 40;

/// What a key is for, read from its prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyKind {
    /// `kc_`: reads published content. Safe to put in a web page.
    Content,
    /// `ki_`: an integration such as `SerpDad`, with scopes.
    Integration,
    /// `ks_`: a staff member's token, with their role.
    Staff,
}

impl KeyKind {
    fn from_prefix(prefix: &str) -> Option<Self> {
        match prefix {
            "kc" => Some(Self::Content),
            "ki" => Some(Self::Integration),
            "ks" => Some(Self::Staff),
            _ => None,
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::Content => "kc",
            Self::Integration => "ki",
            Self::Staff => "ks",
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
    /// A new key of `kind` from 256 random bits and a random identity, read
    /// from the operating system.
    ///
    /// # Errors
    ///
    /// Returns [`GenerateKeyError`] when the operating system has no
    /// randomness to give.
    pub fn generate(kind: KeyKind) -> Result<Self, GenerateKeyError> {
        let mut bytes = [0; RANDOM_BYTES];
        getrandom::fill(&mut bytes).map_err(|source| GenerateKeyError { source })?;
        Ok(Self::from_random(kind, bytes))
    }

    /// The key of `kind` that `bytes` make: the first 8 are its identity,
    /// the other 32 its secret. [`KeyToken::generate`] passes random bytes;
    /// tests pass fixed ones.
    pub fn from_random(kind: KeyKind, bytes: [u8; RANDOM_BYTES]) -> Self {
        let (id, secret) = bytes.split_at(ID_BYTES);
        Self {
            kind,
            id: KeyId(hex(id)),
            secret: Secret::new(hex(secret)),
        }
    }

    /// The whole key as text, to show its owner once. It stays wrapped, so
    /// only an explicit `expose` can print it (RULES.md BL-RUN-5).
    pub fn reveal(self) -> Secret<String> {
        let prefix = self.kind.prefix();
        let id = self.id.0;
        let secret = self.secret.expose();
        Secret::new(format!("{prefix}_{id}_{secret}"))
    }

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

/// `bytes` as lowercase hex digits.
fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .filter_map(|nibble| char::from_digit(u32::from(nibble), 16))
        .collect()
}

/// The operating system could not give random bytes for a new key.
#[derive(Debug, thiserror::Error)]
#[error("no random bytes for a new key")]
pub struct GenerateKeyError {
    #[source]
    source: getrandom::Error,
}

/// Text that is not a key. It says nothing about which part was wrong, so
/// an error never helps a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("this is not a Khalid key")]
pub struct ParseKeyError;
