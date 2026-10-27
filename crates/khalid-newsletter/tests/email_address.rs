//! Email addresses typed into the sign-up form: what is kept, and what is
//! refused before anything is stored or sent.

use khalid_newsletter::EmailAddress;

#[test]
fn an_address_is_kept_trimmed_and_in_lowercase() {
    let address = EmailAddress::parse("  Ada.Lovelace+news@Example.ORG \n").unwrap();
    assert_eq!(address.as_str(), "ada.lovelace+news@example.org");
    assert_eq!(address.to_string(), "ada.lovelace+news@example.org");
}

#[test]
fn the_same_person_typed_twice_is_one_address() {
    let first = EmailAddress::parse("grace@example.com").unwrap();
    let second = EmailAddress::parse("GRACE@example.com").unwrap();
    assert_eq!(first, second);
}

#[test]
fn addresses_that_cannot_receive_mail_are_refused() {
    let too_long_local = format!("{}@example.com", "a".repeat(65));
    let too_long = format!("{}@{}.com", "a".repeat(60), "b".repeat(200));
    let wrong = [
        "",
        "   ",
        "grace",
        "grace@",
        "@example.com",
        "grace@example",
        "grace@@example.com",
        "grace@exa mple.com",
        "gr ace@example.com",
        "grace@-example.com",
        "grace@example-.com",
        "grace@example..com",
        "grace@.example.com",
        "grace@example.com.",
        "grace\u{7}@example.com",
        "<grace>@example.com",
        "grace\"@example.com",
        "grace@exämple.com",
        too_long_local.as_str(),
        too_long.as_str(),
    ];
    for text in wrong {
        assert!(EmailAddress::parse(text).is_err(), "{text:?} was kept");
    }
}

#[test]
fn the_refusal_does_not_repeat_what_was_typed() {
    let error = EmailAddress::parse("private@").unwrap_err();
    assert!(!error.to_string().contains("private"), "{error}");
}
