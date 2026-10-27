//! An answer's headers, and what robots tags ask.

use khalid_audit::{Answer, Robots};

#[test]
fn headers_are_found_without_case_and_html_by_its_type() {
    let answer = Answer::new(200, b"<p>Hi</p>".to_vec())
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_header("X-Robots-Tag", "noindex");
    assert_eq!(answer.header("x-robots-tag"), Some("noindex"));
    assert_eq!(
        answer.header("CONTENT-TYPE"),
        Some("text/html; charset=utf-8")
    );
    assert!(answer.is_html());
    assert!(!Answer::new(200, Vec::new()).is_html());
    let css = Answer::new(200, Vec::new()).with_header("content-type", "text/css");
    assert!(!css.is_html());
}

#[test]
fn robots_words_add_up_across_tags() {
    let none = Robots::default();
    assert!(!none.noindex() && !none.nofollow());
    let tagged = none.with("NoIndex, follow");
    assert!(tagged.noindex() && !tagged.nofollow());
    let both = tagged.with("nofollow");
    assert!(both.noindex() && both.nofollow());
    let all = Robots::default().with("none");
    assert!(all.noindex() && all.nofollow());
    // A word inside another word is not the word.
    assert!(
        !Robots::default()
            .with("max-snippet:-1, noindexed")
            .noindex()
    );
}
