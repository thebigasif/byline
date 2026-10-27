//! Each field a reader fills in is tidied, then kept or refused.

use khalid_comments::{AuthorName, CommentStatus, CommentText, FieldProblem, ReaderEmail};

#[test]
fn a_name_is_one_line_without_control_characters() {
    let name = AuthorName::parse("  Le\u{0}na \n Vogt ").unwrap();
    assert_eq!(name.as_str(), "Lena Vogt");
    assert_eq!(AuthorName::parse(" \u{7} "), Err(FieldProblem::NameMissing));
}

#[test]
fn a_comment_keeps_its_lines_and_tabs_only() {
    let text = CommentText::parse("Great post.\r\n\r\none\u{7}\ttwo\u{1b}  ").unwrap();
    assert_eq!(text.as_str(), "Great post.\n\none\ttwo");
    assert_eq!(CommentText::parse("\n \t"), Err(FieldProblem::TextMissing));
}

#[test]
fn long_fields_are_refused_not_cut_and_counted_in_letters() {
    let too_long = "a".repeat(AuthorName::MAX + 1);
    assert_eq!(AuthorName::parse(&too_long), Err(FieldProblem::NameTooLong));
    let too_long = "ä".repeat(CommentText::MAX + 1);
    assert_eq!(
        CommentText::parse(&too_long),
        Err(FieldProblem::TextTooLong)
    );
    assert!(AuthorName::parse(&"ä".repeat(AuthorName::MAX)).is_ok());
    assert!(CommentText::parse(&"ä".repeat(CommentText::MAX)).is_ok());
}

#[test]
fn an_email_needs_one_at_sign_and_a_dotted_domain() {
    let kept = ReaderEmail::parse(" lena@example.com ").unwrap();
    assert_eq!(kept.as_str(), "lena@example.com");
    let bad = [
        "lena",
        "@example.com",
        "lena@example",
        "lena@.example",
        "le na@example.com",
        "a@b@c.de",
    ];
    for address in bad {
        let read = ReaderEmail::parse(address);
        assert_eq!(read, Err(FieldProblem::EmailInvalid), "{address}");
    }
}

#[test]
fn a_status_reads_back_from_its_name() {
    for status in CommentStatus::ALL {
        assert_eq!(status.as_str().parse::<CommentStatus>(), Ok(status));
    }
    assert!("deleted".parse::<CommentStatus>().is_err());
}
