//! Signing a delivery (RULES.md BL-SEC-7). The header is
//! `Khalid-Signature: t=<unix>,v1=<hex hmac-sha256(key, t + "." + body)>`.

use std::fmt::Write as _;

use chrono::{DateTime, TimeZone, Utc};
use khalid_core::Secret;
use khalid_webhooks::{SigningKey, sign};
use sha2::{Digest, Sha256};

/// A key made up for these tests. It protects nothing.
const KEY: &str = "test-key-generated-for-signature-tests-only";

#[cfg(test)]
fn key(text: &str) -> SigningKey {
    SigningKey::try_new(&Secret::new(text.to_owned())).unwrap()
}

#[cfg(test)]
fn at(unix: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(unix, 0).single().unwrap()
}

#[cfg(test)]
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").unwrap();
        out
    })
}

/// HMAC-SHA256 written out from RFC 2104, independent of the crate under test.
#[cfg(test)]
fn reference_hmac(key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut block = [0_u8; 64];
    block[..key.len()].copy_from_slice(key);
    let pad = |byte: u8| block.iter().map(|k| k ^ byte).collect::<Vec<u8>>();
    let inner = Sha256::new()
        .chain_update(pad(0x36))
        .chain_update(message)
        .finalize();
    Sha256::new()
        .chain_update(pad(0x5c))
        .chain_update(inner)
        .finalize()
        .to_vec()
}

#[test]
fn the_reference_hmac_matches_rfc_4231() {
    let got = reference_hmac(&[0x0b; 20], b"Hi There");
    assert_eq!(
        hex(&got),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
}

#[test]
fn a_signature_is_the_hmac_of_timestamp_dot_body() {
    let body = br#"{"event":"post.published"}"#;
    let signature = sign(&key(KEY), at(1_793_001_600), body);
    let want = reference_hmac(KEY.as_bytes(), b"1793001600.{\"event\":\"post.published\"}");
    assert_eq!(signature.timestamp(), 1_793_001_600);
    assert_eq!(
        signature.to_string(),
        format!("t=1793001600,v1={}", hex(&want))
    );
}

#[test]
fn a_different_body_time_or_key_gives_a_different_signature() {
    let base = sign(&key(KEY), at(1_793_001_600), b"hello");
    let other_key = key("another-test-key-generated-for-these-tests");
    assert_ne!(base, sign(&key(KEY), at(1_793_001_600), b"hellO"));
    assert_ne!(base, sign(&key(KEY), at(1_793_001_601), b"hello"));
    assert_ne!(base, sign(&other_key, at(1_793_001_600), b"hello"));
}

#[test]
fn a_short_key_is_refused() {
    let error = SigningKey::try_new(&Secret::new("short".to_owned())).unwrap_err();
    assert_eq!(
        error.to_string(),
        "a webhook signing key needs at least 32 bytes, and this one has 5"
    );
}

#[test]
fn the_key_never_appears_in_debug_output() {
    let debug = format!("{:?}", key(KEY));
    assert_eq!(debug, "SigningKey([redacted])");
}
