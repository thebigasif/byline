//! The tokens in confirmation and unsubscribe links: 256 random bits,
//! written as text for a link, and stored only as their hash.

use khalid_newsletter::{SubscriberToken, TokenHash};

#[test]
fn a_new_token_is_64_hex_digits_and_never_printed() {
    let token = SubscriberToken::generate().unwrap();
    let text = token.expose_text();
    assert_eq!(text.len(), 64);
    assert!(
        text.bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    );
    assert_eq!(format!("{token:?}"), "SubscriberToken([redacted])");
}

#[test]
fn two_new_tokens_differ() {
    let first = SubscriberToken::generate().unwrap();
    let second = SubscriberToken::generate().unwrap();
    assert_ne!(first.hash(), second.hash());
}

#[test]
fn a_token_read_back_from_its_link_has_the_same_hash() {
    let token = SubscriberToken::generate().unwrap();
    let read = SubscriberToken::parse(&token.expose_text()).unwrap();
    assert_eq!(read.hash(), token.hash());
}

#[test]
fn the_hash_is_the_sha256_of_the_token_text() {
    let token = SubscriberToken::parse(&"0".repeat(64)).unwrap();
    // SHA-256 of 64 ASCII zeros, from `printf '%064d' 0 | sha256sum`.
    let expected = "60e05bd1b195af2f94112fa7197a5c88289058840ce7c6df9693756bc6250f55";
    let expected: Vec<u8> = (0..32)
        .map(|at| u8::from_str_radix(expected.get(at * 2..at * 2 + 2).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(token.hash().as_bytes().as_slice(), expected);
}

#[test]
fn text_that_is_not_a_token_is_refused() {
    let upper = "A".repeat(64);
    for text in [
        "",
        "abc",
        &"g".repeat(64),
        &"0".repeat(63),
        &"0".repeat(65),
        &upper,
    ] {
        assert!(SubscriberToken::parse(text).is_err(), "{text:?} was read");
    }
}

#[test]
fn a_hash_is_rebuilt_from_its_32_bytes() {
    let token = SubscriberToken::generate().unwrap();
    let bytes = *token.hash().as_bytes();
    assert_eq!(TokenHash::from_bytes(bytes), token.hash());
}
