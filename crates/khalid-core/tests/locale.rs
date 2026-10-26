//! The `Locale` rules: a language, an optional script and an optional region.

use khalid_core::{Locale, ParseLocaleError};
use proptest::prelude::{prop_assert_eq, proptest};

#[test]
fn accepts_the_tags_sites_use_and_writes_them_in_standard_case() {
    let cases = [
        ("en", "en"),
        ("de", "de"),
        ("pt-BR", "pt-BR"),
        ("pt-br", "pt-BR"),
        ("EN-gb", "en-GB"),
        ("zh-hant", "zh-Hant"),
        ("zh-Hant-TW", "zh-Hant-TW"),
        ("es-419", "es-419"),
    ];
    for (input, want) in cases {
        let locale = Locale::try_new(input).unwrap();
        assert_eq!(locale.as_str(), want, "input {input:?}");
        assert_eq!(input.parse::<Locale>().unwrap(), locale);
        assert_eq!(locale.language(), want.split('-').next().unwrap());
    }
}

#[test]
fn refuses_what_is_not_a_language_tag() {
    let cases = [
        ("", ParseLocaleError::Empty),
        ("english", ParseLocaleError::Language),
        ("e", ParseLocaleError::Language),
        ("e1", ParseLocaleError::Language),
        ("en_GB", ParseLocaleError::Language),
        (
            "en-",
            ParseLocaleError::Subtag {
                subtag: String::new(),
            },
        ),
        (
            "en-Latn-Latn",
            ParseLocaleError::Subtag {
                subtag: "Latn".to_owned(),
            },
        ),
        (
            "en-GB-US",
            ParseLocaleError::Subtag {
                subtag: "US".to_owned(),
            },
        ),
        (
            "en-x1",
            ParseLocaleError::Subtag {
                subtag: "x1".to_owned(),
            },
        ),
    ];
    for (input, want) in cases {
        assert_eq!(Locale::try_new(input), Err(want), "input {input:?}");
    }
}

#[test]
fn errors_read_as_plain_phrases() {
    let error = Locale::try_new("english").unwrap_err().to_string();
    assert_eq!(
        error,
        "the language must be a two- or three-letter code such as en or de"
    );
}

proptest! {
    #[test]
    fn standard_case_is_stable(tag in "[a-zA-Z]{2,3}(-[a-zA-Z]{4})?(-([a-zA-Z]{2}|[0-9]{3}))?") {
        let once = Locale::try_new(&tag).unwrap();
        let twice = Locale::try_new(once.as_str()).unwrap();
        prop_assert_eq!(once.as_str(), twice.as_str());
        prop_assert_eq!(once.as_str().to_lowercase(), tag.to_lowercase());
    }
}
