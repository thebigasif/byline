//! `flags_slug_part`: the parts of a web address the slug rules flag, which
//! `khalid lint --fix` removes.

use khalid_lint::{RuleId, flags_slug_part};

#[test]
fn filler_words_are_flagged_by_language() {
    let stopwords = RuleId::new("slug-stopwords");
    assert!(flags_slug_part(stopwords, "en", "the"));
    assert!(flags_slug_part(stopwords, "de", "und"));
    assert!(!flags_slug_part(stopwords, "en", "blog"));
    assert!(!flags_slug_part(stopwords, "de", "the"));
}

#[test]
fn years_are_flagged_and_other_numbers_are_not() {
    let date = RuleId::new("slug-date");
    assert!(flags_slug_part(date, "en", "2026"));
    assert!(flags_slug_part(date, "en", "1999"));
    assert!(!flags_slug_part(date, "en", "1080"));
    assert!(!flags_slug_part(date, "en", "20260"));
    assert!(!flags_slug_part(RuleId::new("long-slug"), "en", "the"));
}
