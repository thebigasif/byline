//! A post password is hashed with a fresh salt, checked against its hash,
//! and never printed.

use khalid_api::post_password::{NewPassword, password_matches};
use khalid_core::PasswordHash;

/// A password made up for these tests.
const PASSWORD: &str = "generated-for-this-test";

#[test]
fn a_hash_matches_its_password_and_nothing_else() {
    let password = NewPassword::try_new(PASSWORD.to_owned()).unwrap();
    let hash = password.hash().unwrap();
    assert!(hash.as_str().starts_with("$argon2id$"), "{hash:?}");
    assert!(password_matches(&hash, PASSWORD));
    assert!(!password_matches(&hash, "generated-for-another-test"));
    assert_ne!(password.hash().unwrap(), hash, "each hash has its own salt");
}

#[test]
fn a_hash_that_cannot_be_read_matches_nothing() {
    let broken = PasswordHash::try_new("$argon2id$not-a-hash").unwrap();
    assert!(!password_matches(&broken, PASSWORD));
}

#[test]
fn a_password_has_8_to_128_characters_and_is_never_printed() {
    assert!(NewPassword::try_new("seven77".to_owned()).is_err());
    assert!(NewPassword::try_new("x".repeat(129)).is_err());
    assert!(NewPassword::try_new("naïve-café".to_owned()).is_ok());
    let password = NewPassword::try_new(PASSWORD.to_owned()).unwrap();
    assert!(!format!("{password:?}").contains(PASSWORD));
    let read: NewPassword = serde_json::from_str("\"generated-for-this-test\"").unwrap();
    assert_eq!(read, password);
    assert!(serde_json::from_str::<NewPassword>("\"short\"").is_err());
}
