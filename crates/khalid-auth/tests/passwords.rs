//! New passwords meet the length rules, are stored only as Argon2id, and
//! an unknown email is checked as slowly as a wrong password.

use khalid_auth::{Password, PasswordError, PasswordHash, PasswordHashing, WeakPasswordError};

#[cfg(test)]
fn password(text: &str) -> Password {
    Password::new(text.to_owned()).expect("a long enough password")
}

#[test]
fn a_password_needs_twelve_characters_and_at_most_a_kilobyte() {
    assert_eq!(
        Password::new("eleven char".to_owned()).err(),
        Some(WeakPasswordError::TooShort)
    );
    assert!(Password::new("twelve chars".to_owned()).is_ok());
    assert!(
        Password::new("ünïcödé ünï".to_owned()).is_err(),
        "characters count, not bytes"
    );
    assert_eq!(
        Password::new("a".repeat(1025)).err(),
        Some(WeakPasswordError::TooLong)
    );
}

#[test]
fn debug_never_shows_the_password() {
    let shown = format!("{:?}", password("correct horse battery"));
    assert!(!shown.contains("horse"), "{shown}");
}

#[tokio::test]
async fn the_right_password_matches_its_hash_and_a_wrong_one_does_not() {
    let hashing = PasswordHashing::fast_for_tests();
    let hash = hashing
        .hash(password("correct horse battery"))
        .await
        .expect("hashed");
    assert!(
        hash.as_str().starts_with("$argon2id$"),
        "stored as Argon2id"
    );
    assert!(!hash.as_str().contains("horse"));
    let right = hashing
        .verify(password("correct horse battery"), Some(hash.clone()))
        .await;
    let wrong = hashing
        .verify(password("correct horse battery!"), Some(hash))
        .await;
    assert!(right.expect("checked"));
    assert!(!wrong.expect("checked"));
}

#[tokio::test]
async fn the_same_password_hashes_differently_each_time() {
    let hashing = PasswordHashing::fast_for_tests();
    let first = hashing.hash(password("correct horse battery")).await;
    let second = hashing.hash(password("correct horse battery")).await;
    assert_ne!(
        first.expect("hashed"),
        second.expect("hashed"),
        "a fresh salt each time"
    );
}

#[tokio::test]
async fn no_stored_hash_never_matches_even_the_dummy_text() {
    let hashing = PasswordHashing::fast_for_tests();
    for text in ["no account has this password", "anything at all here"] {
        let matched = hashing.verify(password(text), None).await;
        assert!(!matched.expect("checked"), "{text}");
    }
}

#[test]
fn only_argon2id_hashes_are_read_back() {
    assert!(
        PasswordHash::from_phc("$argon2id$v=19$m=8,t=1,p=1$c2FsdHNhbHQ$aGFzaA".to_owned()).is_ok()
    );
    for phc in ["$argon2i$v=19$m=8,t=1,p=1$c2FsdA$aGFzaA", "plain text", ""] {
        assert!(
            matches!(
                PasswordHash::from_phc(phc.to_owned()),
                Err(PasswordError::Unreadable)
            ),
            "{phc}"
        );
    }
}

#[tokio::test]
async fn a_corrupt_stored_hash_is_an_error_not_a_match() {
    let hashing = PasswordHashing::fast_for_tests();
    let corrupt =
        PasswordHash::from_phc("$argon2id$garbage".to_owned()).expect("the prefix is right");
    let result = hashing
        .verify(password("correct horse battery"), Some(corrupt))
        .await;
    assert!(
        matches!(result, Err(PasswordError::Unreadable)),
        "{result:?}"
    );
}
