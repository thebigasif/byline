//! Verifying an incoming webhook (RULES.md BL-SEC-7): the signature must
//! match the body and key, and its time must be close to now.

use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use khalid_core::{FixedClock, Secret};
use khalid_webhooks::{SigningKey, Verifier, VerifyError, sign};
use proptest::prelude::{any, prop_assert, prop_assert_eq, proptest};

/// A key made up for these tests. It protects nothing.
const KEY: &str = "test-key-generated-for-verifier-tests-only";

/// The time every signature in these tests is made.
const SIGNED: i64 = 1_793_001_600;

#[cfg(test)]
fn key(text: &str) -> SigningKey {
    SigningKey::try_new(&Secret::new(text.to_owned())).unwrap()
}

#[cfg(test)]
fn at(unix: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(unix, 0).single().unwrap()
}

#[cfg(test)]
fn clock(unix: i64) -> FixedClock {
    FixedClock::new(at(unix))
}

#[cfg(test)]
fn verifier() -> Verifier {
    Verifier::new(key(KEY))
}

#[cfg(test)]
fn signed(body: &[u8]) -> String {
    sign(&key(KEY), at(SIGNED), body).to_string()
}

#[test]
fn a_fresh_signature_verifies() {
    let header = signed(b"hello");
    assert_eq!(
        verifier().verify(&header, b"hello", &clock(SIGNED + 60)),
        Ok(())
    );
}

#[test]
fn a_changed_body_is_refused() {
    let error = verifier()
        .verify(&signed(b"hello"), b"hellO", &clock(SIGNED))
        .unwrap_err();
    assert_eq!(error, VerifyError::Mismatch);
}

#[test]
fn a_signature_from_another_key_is_refused() {
    let other = key("another-test-key-generated-for-these-tests");
    let header = sign(&other, at(SIGNED), b"hello").to_string();
    let error = verifier()
        .verify(&header, b"hello", &clock(SIGNED))
        .unwrap_err();
    assert_eq!(error, VerifyError::Mismatch);
}

#[test]
fn an_old_signature_is_refused_as_a_replay() {
    let error = verifier()
        .verify(&signed(b"hello"), b"hello", &clock(SIGNED + 301))
        .unwrap_err();
    assert_eq!(error, VerifyError::Stale { age_seconds: 301 });
}

#[test]
fn a_signature_from_the_future_is_refused() {
    let error = verifier()
        .verify(&signed(b"hello"), b"hello", &clock(SIGNED - 301))
        .unwrap_err();
    assert_eq!(error, VerifyError::Stale { age_seconds: -301 });
}

#[test]
fn the_edge_of_the_window_still_verifies() {
    for now in [SIGNED - 300, SIGNED + 300] {
        assert_eq!(
            verifier().verify(&signed(b"hello"), b"hello", &clock(now)),
            Ok(()),
            "now {now}"
        );
    }
}

#[test]
fn the_tolerance_can_be_widened() {
    let wide = verifier().with_tolerance(Duration::from_hours(1));
    assert_eq!(
        wide.verify(&signed(b"hello"), b"hello", &clock(SIGNED + 3000)),
        Ok(())
    );
}

#[test]
fn any_matching_v1_among_several_verifies() {
    let good = signed(b"hello");
    let (_, v1) = good.split_once(",v1=").unwrap();
    let header = format!("t={SIGNED},v1={},v1={v1}", "00".repeat(32));
    assert_eq!(verifier().verify(&header, b"hello", &clock(SIGNED)), Ok(()));
}

#[test]
fn malformed_headers_are_refused() {
    for header in [
        "",
        "v1=abcd",
        "t=1793001600",
        "t=soon,v1=abcd",
        "t=1793001600,v1=not-hex",
        "t=1793001600,v1=abc",
        "t=1793001600,t=1793001600,v1=abcd",
    ] {
        let error = verifier()
            .verify(header, b"hello", &clock(SIGNED))
            .unwrap_err();
        assert_eq!(error, VerifyError::Malformed, "{header:?}");
    }
}

#[test]
fn the_key_never_appears_in_debug_output() {
    let debug = format!("{:?}", verifier());
    assert!(!debug.contains(KEY), "{debug}");
}

proptest! {
    #[test]
    fn whatever_is_signed_verifies_and_any_change_does_not(
        body in proptest::collection::vec(any::<u8>(), 0..512),
        flip in any::<usize>(),
        unix in 0_i64..4_000_000_000,
    ) {
        let header = sign(&key(KEY), at(unix), &body).to_string();
        prop_assert_eq!(verifier().verify(&header, &body, &clock(unix)), Ok(()));
        let mut changed = body;
        if changed.is_empty() {
            changed.push(0);
        } else {
            let index = flip % changed.len();
            changed[index] ^= 1;
        }
        prop_assert!(verifier().verify(&header, &changed, &clock(unix)).is_err());
    }

    #[test]
    fn no_header_text_makes_the_verifier_panic(header in ".{0,200}") {
        let _ = verifier().verify(&header, b"hello", &clock(SIGNED));
    }
}
