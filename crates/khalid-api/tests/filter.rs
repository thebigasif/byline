//! The `filter` grammar, Ghost's NQL: every operator, `+`, `,` and brackets, and mistakes.

use khalid_api::query::{Condition, Filter, FilterErrorKind, Literal, Predicate};
use proptest::prelude::{prop_assert, proptest};

#[cfg(test)]
fn parse(text: &str) -> Filter {
    text.parse::<Filter>().unwrap()
}

#[cfg(test)]
fn condition(field: &str, predicate: Predicate) -> Filter {
    Filter::Condition(Condition::new(field, predicate))
}

#[cfg(test)]
fn text(value: &str) -> Literal {
    Literal::Text(value.to_owned())
}

#[test]
fn a_condition_is_equality_and_a_minus_negates() {
    let cases = [
        ("tag:news", "tag", Predicate::Is(text("news"))),
        ("tag:-draft", "tag", Predicate::IsNot(text("draft"))),
        (
            "authors.slug:asif",
            "authors.slug",
            Predicate::Is(text("asif")),
        ),
    ];
    for (input, field, predicate) in cases {
        assert_eq!(parse(input), condition(field, predicate), "{input}");
    }
}

#[test]
fn words_become_keywords_numbers_or_text_and_quotes_keep_text() {
    let cases = [
        ("null", Literal::Null),
        ("true", Literal::Bool(true)),
        ("5", Literal::Integer(5)),
        ("how-to-link.v2", text("how-to-link.v2")),
        ("99999999999999999999", text("99999999999999999999")),
        ("'Don\\'t panic'", text("Don't panic")),
        ("'true'", text("true")),
        ("'2026-12-31 23:59:59'", text("2026-12-31 23:59:59")),
        ("'a\\\\b'", text("a\\b")),
        ("''", text("")),
    ];
    for (value, want) in cases {
        let got = parse(&format!("field:{value}"));
        assert_eq!(got, condition("field", Predicate::Is(want)), "{value}");
    }
}

#[test]
fn every_operator_from_the_api_standard() {
    let list = || vec![text("seo"), text("links")];
    let cases = [
        ("tag:[seo,links]", Predicate::In(list())),
        ("tag:-[seo,links]", Predicate::NotIn(list())),
        ("at:>'2026-01-01'", Predicate::Greater(text("2026-01-01"))),
        ("at:>=5", Predicate::GreaterOrEqual(Literal::Integer(5))),
        ("at:<10", Predicate::Less(Literal::Integer(10))),
        (
            "at:<='2026-12-31'",
            Predicate::LessOrEqual(text("2026-12-31")),
        ),
        (
            "at:~'internal links'",
            Predicate::Contains("internal links".into()),
        ),
        ("at:-~draft", Predicate::NotContains("draft".into())),
        ("at:~^'how-'", Predicate::StartsWith("how-".into())),
        ("at:~$'-guide'", Predicate::EndsWith("-guide".into())),
    ];
    for (input, predicate) in cases {
        let Filter::Condition(got) = parse(input) else {
            panic!("{input}")
        };
        assert_eq!(got.predicate(), &predicate, "{input}");
    }
}

#[test]
fn and_binds_tighter_than_or_and_brackets_group() {
    let news = || condition("tag", Predicate::Is(text("news")));
    let featured = || condition("featured", Predicate::Is(Literal::Bool(true)));
    let author = || condition("author", Predicate::Is(text("asif")));
    assert_eq!(
        parse("tag:news+featured:true"),
        Filter::And(vec![news(), featured()])
    );
    assert_eq!(
        parse("tag:news+featured:true,author:asif"),
        Filter::Or(vec![Filter::And(vec![news(), featured()]), author()])
    );
    assert_eq!(
        parse("tag:news+(featured:true,author:asif)"),
        Filter::And(vec![news(), Filter::Or(vec![featured(), author()])])
    );
    assert_eq!(parse("((tag:news))"), news());
}

#[test]
fn mistakes_are_refused_with_where_and_what() {
    let cases = [
        ("", 0, FilterErrorKind::Empty),
        ("tag", 3, FilterErrorKind::ExpectedColon),
        ("tag:", 4, FilterErrorKind::ExpectedValue),
        ("tag:-", 5, FilterErrorKind::ExpectedValue),
        (":news", 0, FilterErrorKind::ExpectedField),
        ("9tag:news", 0, FilterErrorKind::ExpectedField),
        ("tag:'news", 4, FilterErrorKind::UnclosedQuote),
        ("tag:'news\\", 4, FilterErrorKind::UnclosedQuote),
        ("tag:news)", 8, FilterErrorKind::Unexpected(')')),
        ("tag:[seo,links", 14, FilterErrorKind::UnclosedBracket),
        ("tag:[seo;links]", 8, FilterErrorKind::Unexpected(';')),
        ("tag:[]", 5, FilterErrorKind::ExpectedValue),
        ("title:~5", 7, FilterErrorKind::ExpectedText),
        ("(tag:news", 9, FilterErrorKind::UnclosedBracket),
        ("tag:news+", 9, FilterErrorKind::ExpectedField),
        ("tag:news,,featured:true", 9, FilterErrorKind::ExpectedField),
    ];
    for (input, at, kind) in cases {
        let error = input.parse::<Filter>().unwrap_err();
        assert_eq!((error.position(), error.kind()), (at, &kind), "{input:?}");
    }
}

#[test]
fn the_error_message_names_the_place() {
    let error = "tag:".parse::<Filter>().unwrap_err();
    assert_eq!(error.to_string(), "the filter needs a value at character 5");
}

#[test]
fn limits_stop_hostile_filters() {
    let long = format!("title:'{}'", "a".repeat(5000));
    let deep = format!("{}tag:news{}", "(".repeat(40), ")".repeat(40));
    let wide = vec!["tag:news"; 200].join("+");
    for (input, kind) in [
        (long, FilterErrorKind::TooLong),
        (deep, FilterErrorKind::TooDeep),
        (wide, FilterErrorKind::TooManyConditions),
    ] {
        let error = input.parse::<Filter>().unwrap_err();
        assert_eq!(error.kind(), &kind);
    }
    let ten_deep = format!("{}tag:news{}", "(".repeat(10), ")".repeat(10));
    assert!(ten_deep.parse::<Filter>().is_ok());
}

proptest! {
    #[test]
    fn no_input_makes_the_parser_panic(input in "\\PC{0,300}") {
        let _ = input.parse::<Filter>();
    }

    #[test]
    fn errors_point_inside_the_input(input in "[a-z_.:+,()\\[\\]'~^$<>=0-9-]{0,120}") {
        if let Err(error) = input.parse::<Filter>() {
            prop_assert!(error.position() <= input.chars().count());
        }
    }
}
