//! The `Slug` rules, case by case and as properties.

use khalid_core::{ParseSlugError, Slug};
use proptest::prelude::{prop_assert, prop_assert_eq, proptest};

#[test]
fn accepts_lowercase_words_digits_and_single_hyphens() {
    for text in ["seo", "why-internal-links-matter", "top-10-tools-2026", "a"] {
        let slug = Slug::try_new(text).unwrap();
        assert_eq!(slug.as_str(), text);
        assert_eq!(slug.to_string(), text);
        assert_eq!(text.parse::<Slug>().unwrap(), slug);
    }
}

#[test]
fn refuses_empty_text() {
    assert_eq!(Slug::try_new(""), Err(ParseSlugError::Empty));
}

#[test]
fn refuses_text_over_the_limit_and_accepts_the_limit() {
    let at_limit = "a".repeat(Slug::MAX_LEN);
    assert!(Slug::try_new(&at_limit).is_ok());
    let over = "a".repeat(Slug::MAX_LEN + 1);
    assert_eq!(
        Slug::try_new(&over),
        Err(ParseSlugError::TooLong {
            len: Slug::MAX_LEN + 1
        })
    );
}

#[test]
fn names_the_first_character_that_is_not_allowed() {
    let cases = [
        ("Links", 'L', 0),
        ("internal links", ' ', 8),
        ("über-seo", 'ü', 0),
        ("seo_tips", '_', 3),
        ("seo/tips", '/', 3),
    ];
    for (text, character, position) in cases {
        assert_eq!(
            Slug::try_new(text),
            Err(ParseSlugError::InvalidCharacter {
                character,
                position
            }),
            "text {text:?}"
        );
    }
}

#[test]
fn refuses_a_hyphen_at_either_end() {
    for text in ["-seo", "seo-", "-"] {
        assert_eq!(
            Slug::try_new(text),
            Err(ParseSlugError::EdgeHyphen),
            "text {text:?}"
        );
    }
}

#[test]
fn refuses_two_hyphens_in_a_row() {
    assert_eq!(
        Slug::try_new("internal--links"),
        Err(ParseSlugError::DoubleHyphen { position: 8 })
    );
}

#[test]
fn errors_read_as_plain_lowercase_phrases() {
    let error = Slug::try_new("Links").unwrap_err().to_string();
    assert_eq!(
        error,
        "the slug has 'L' at byte 0, but only a to z, 0 to 9 and hyphens are allowed"
    );
}

proptest! {
    #[test]
    fn every_well_formed_slug_round_trips(text in "[a-z0-9]{1,12}(-[a-z0-9]{1,12}){0,6}") {
        let slug = Slug::try_new(&text).unwrap();
        prop_assert_eq!(slug.as_str(), text.as_str());
    }

    #[test]
    fn nothing_outside_the_alphabet_gets_through(text in "\\PC{0,40}") {
        let well_formed = !text.is_empty()
            && text.len() <= Slug::MAX_LEN
            && text.split('-').all(|word| {
                !word.is_empty() && word.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            });
        prop_assert!(Slug::try_new(&text).is_ok() == well_formed, "text {:?}", text);
    }
}

#[test]
fn reads_from_text_in_both_forms_and_refuses_a_bad_one() {
    let ascii: Slug = serde_json::from_str("\"why-links-matter\"").unwrap();
    assert_eq!(ascii.as_str(), "why-links-matter");
    let unicode: Slug = serde_json::from_str("\"über-größe\"").unwrap();
    assert_eq!(unicode.as_str(), "über-größe");
    let error = serde_json::from_str::<Slug>("\"Why Links\"")
        .unwrap_err()
        .to_string();
    assert!(error.starts_with("the slug has 'W' at byte 0"), "{error}");
}
