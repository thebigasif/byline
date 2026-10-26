//! Slugs made from titles: ASCII by default with transliteration by language,
//! Unicode where ASCII is unreadable or the site asks for it
//! (`docs/decisions/2026-10-26-cto-answers.md`, section 2).

use khalid_core::{Locale, ParseSlugError, Slug, SlugMode};

#[cfg(test)]
fn slug(title: &str, tag: &str, mode: SlugMode) -> String {
    let locale = Locale::try_new(tag).unwrap();
    Slug::from_title(title, &locale, mode)
        .unwrap()
        .as_str()
        .to_owned()
}

#[test]
fn english_titles_become_lowercase_words_and_hyphens() {
    let cases = [
        ("Why Internal Links Matter", "why-internal-links-matter"),
        (
            "Don't panic: 10 SEO tips for 2026!",
            "dont-panic-10-seo-tips-for-2026",
        ),
        ("  Title tags -- a guide  ", "title-tags-a-guide"),
    ];
    for (title, want) in cases {
        assert_eq!(slug(title, "en", SlugMode::Ascii), want, "title {title:?}");
    }
}

#[test]
fn german_umlauts_and_sharp_s_follow_the_german_convention() {
    assert_eq!(
        slug("Über Größe und Straßen", "de", SlugMode::Ascii),
        "ueber-groesse-und-strassen"
    );
    assert_eq!(
        slug("ÄRGER MIT ÖL", "de", SlugMode::Ascii),
        "aerger-mit-oel"
    );
}

#[test]
fn other_latin_scripts_lose_their_accents() {
    assert_eq!(
        slug("Café crème à la française", "fr", SlugMode::Ascii),
        "cafe-creme-a-la-francaise"
    );
    assert_eq!(
        slug("Łódź: żółć i gęśla", "pl", SlugMode::Ascii),
        "lodz-zolc-i-gesla"
    );
    assert_eq!(slug("Über", "en", SlugMode::Ascii), "uber");
}

#[test]
fn scripts_without_an_ascii_form_keep_their_letters() {
    assert_eq!(
        slug("تحسين محركات البحث", "ar", SlugMode::Ascii),
        "تحسين-محركات-البحث"
    );
    assert_eq!(
        slug("検索エンジン最適化 入門", "ja", SlugMode::Ascii),
        "検索エンジン最適化-入門"
    );
}

#[test]
fn a_unicode_site_keeps_accents_in_lowercase() {
    assert_eq!(slug("Über Größe", "de", SlugMode::Unicode), "über-größe");
    assert_eq!(slug("Café Crème", "fr", SlugMode::Unicode), "café-crème");
    assert_eq!(
        slug("検索エンジン", "ja", SlugMode::Unicode),
        "検索エンジン"
    );
}

#[test]
fn long_titles_are_cut_at_a_word_within_the_limit() {
    let title = "internal links ".repeat(30);
    let made = slug(&title, "en", SlugMode::Ascii);
    assert!(made.len() <= Slug::MAX_LEN, "{} bytes", made.len());
    assert!(made.ends_with("links"), "{made}");
}

#[test]
fn the_unicode_limit_counts_the_percent_encoded_bytes() {
    let title = "最適化 ".repeat(40);
    let made = slug(&title, "ja", SlugMode::Unicode);
    let encoded: usize = made
        .chars()
        .map(|c| if c.is_ascii() { 1 } else { c.len_utf8() * 3 })
        .sum();
    assert!(encoded <= Slug::MAX_LEN, "{encoded} encoded bytes");
    assert!(made.ends_with("最適化"), "{made}");
}

#[test]
fn compatibility_letters_take_their_plain_form() {
    assert_eq!(slug("𝒩ew ﬁndings", "en", SlugMode::Ascii), "new-findings");
}

#[test]
fn a_title_with_nothing_to_keep_is_refused() {
    let locale = Locale::try_new("en").unwrap();
    assert_eq!(
        Slug::from_title("?!", &locale, SlugMode::Ascii),
        Err(ParseSlugError::Empty)
    );
}

#[test]
fn unicode_slugs_are_checked_too() {
    assert!(Slug::try_new_unicode("über-größe").is_ok());
    assert!(Slug::try_new_unicode("検索エンジン").is_ok());
    assert!(matches!(
        Slug::try_new_unicode("Über"),
        Err(ParseSlugError::InvalidCharacter {
            character: 'Ü',
            position: 0
        })
    ));
    assert_eq!(
        Slug::try_new_unicode("über--größe"),
        Err(ParseSlugError::DoubleHyphen { position: 5 })
    );
    assert!(Slug::try_new("über").is_err(), "try_new stays strict ASCII");
}

proptest::proptest! {
    #[test]
    fn any_title_gives_a_slug_within_the_limit_or_is_refused_as_empty(title in "\\PC{0,300}") {
        let locale = Locale::try_new("de").unwrap();
        for mode in [SlugMode::Ascii, SlugMode::Unicode] {
            match Slug::from_title(&title, &locale, mode) {
                Ok(slug) => {
                    let encoded: usize = slug.as_str().bytes().map(|b| if b.is_ascii() { 1 } else { 3 }).sum();
                    proptest::prop_assert!(encoded <= Slug::MAX_LEN, "title {:?}", title);
                }
                Err(error) => proptest::prop_assert_eq!(error, ParseSlugError::Empty, "title {:?}", title),
            }
        }
    }
}
