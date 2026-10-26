//! The verifier for incoming webhooks.

use std::time::Duration;

use hmac::Mac;
use khalid_core::Clock;

use crate::SigningKey;
use crate::signature::MAC_BYTES;

/// How far a signature's time may be from the receiver's clock: five
/// minutes, which absorbs clock drift and slow networks but makes a
/// recorded delivery useless for replay soon after (ADR 0010).
const DEFAULT_TOLERANCE: Duration = Duration::from_mins(5);

/// Checks the `Khalid-Signature` header of an incoming webhook.
///
/// A header passes when its timestamp is within the tolerance of the clock
/// (five minutes unless changed) and any `v1` value matches. Several `v1`
/// values let a sender rotate keys. Each comparison takes the same time
/// whichever byte differs, so timing reveals nothing about the key.
///
/// # Examples
///
/// ```
/// use chrono::DateTime;
/// use khalid_core::{FixedClock, Secret};
/// use khalid_webhooks::{SigningKey, Verifier, sign};
///
/// let key = SigningKey::try_new(&Secret::new("made-up-key-for-this-example-only".to_owned()))?;
/// let at = DateTime::from_timestamp(1_793_001_600, 0).ok_or("no such time")?;
/// let header = sign(&key, at, b"{}").to_string();
/// Verifier::new(key).verify(&header, b"{}", &FixedClock::new(at))?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub struct Verifier {
    key: SigningKey,
    tolerance: Duration,
}

impl Verifier {
    /// A verifier for deliveries signed with `key`.
    pub fn new(key: SigningKey) -> Self {
        Self {
            key,
            tolerance: DEFAULT_TOLERANCE,
        }
    }

    /// The same verifier, accepting timestamps up to `tolerance` from now.
    #[must_use]
    pub fn with_tolerance(self, tolerance: Duration) -> Self {
        Self { tolerance, ..self }
    }

    /// Checks `header` against `body` at the time `clock` reads.
    ///
    /// # Errors
    ///
    /// Returns [`VerifyError::Malformed`] when the header is not
    /// `t=<unix>,v1=<64 hex digits>`, [`VerifyError::Stale`] when its time is
    /// outside the tolerance, and [`VerifyError::Mismatch`] when no `v1`
    /// value matches.
    pub fn verify(&self, header: &str, body: &[u8], clock: &dyn Clock) -> Result<(), VerifyError> {
        let (timestamp, candidates) = parse(header).ok_or(VerifyError::Malformed)?;
        let age_seconds = clock.now().timestamp().saturating_sub(timestamp);
        if age_seconds.unsigned_abs() > self.tolerance.as_secs() {
            return Err(VerifyError::Stale { age_seconds });
        }
        let expected = self.key.mac(timestamp, body);
        if candidates
            .iter()
            .any(|candidate| expected.clone().verify_slice(candidate).is_ok())
        {
            Ok(())
        } else {
            Err(VerifyError::Mismatch)
        }
    }
}

/// Why an incoming webhook was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum VerifyError {
    /// The header is missing its parts or they are not well formed.
    #[error("the signature header is not in the form t=<unix>,v1=<hex>")]
    Malformed,
    /// The signature's time is too far from now, so it may be a replay.
    #[error("the signature was made {age_seconds} seconds from now, outside the allowed window")]
    Stale {
        /// Seconds between the signature's time and now. Negative when the
        /// signature claims a time in the future.
        age_seconds: i64,
    },
    /// The signature does not match the body and key.
    #[error("the signature does not match the body")]
    Mismatch,
}

/// Splits a header into its one timestamp and its `v1` values. Unknown keys
/// are skipped, so a later scheme can be added beside `v1`.
fn parse(header: &str) -> Option<(i64, Vec<[u8; MAC_BYTES]>)> {
    let mut timestamp = None;
    let mut candidates = Vec::new();
    for part in header.split(',') {
        let (name, value) = part.trim().split_once('=')?;
        match name {
            "t" if timestamp.is_none() => timestamp = Some(value.parse::<i64>().ok()?),
            "t" => return None,
            "v1" => candidates.push(decode_hex(value)?),
            _ => {}
        }
    }
    let timestamp = timestamp?;
    (!candidates.is_empty()).then_some((timestamp, candidates))
}

/// Reads exactly 64 hex digits into 32 bytes.
fn decode_hex(text: &str) -> Option<[u8; MAC_BYTES]> {
    let (pairs, rest) = text.as_bytes().as_chunks::<2>();
    if pairs.len() != MAC_BYTES || !rest.is_empty() {
        return None;
    }
    let mut out = [0_u8; MAC_BYTES];
    for (slot, &[high, low]) in out.iter_mut().zip(pairs) {
        *slot = nibble(high)? << 4 | nibble(low)?;
    }
    Some(out)
}

/// The value of one hex digit.
fn nibble(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_reads_both_cases_and_nothing_else() {
        let lower = format!("{}0a1b", "00ff7a".repeat(10));
        let mut want = [0_u8; MAC_BYTES];
        for (slot, byte) in want.iter_mut().zip([0x00, 0xff, 0x7a].iter().cycle()) {
            *slot = *byte;
        }
        want[30] = 0x0a;
        want[31] = 0x1b;
        assert_eq!(decode_hex(&lower), Some(want));
        assert_eq!(decode_hex(&lower.to_uppercase()), Some(want));
        assert_eq!(decode_hex(&lower.replace('f', "g")), None);
        assert_eq!(decode_hex(lower.trim_start_matches('0')), None);
    }

    #[test]
    fn unknown_parts_are_skipped() {
        let header = format!("t=5,v2=whatever,v1={}", "ab".repeat(MAC_BYTES));
        assert_eq!(parse(&header).map(|(t, v)| (t, v.len())), Some((5, 1)));
    }
}
