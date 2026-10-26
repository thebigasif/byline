//! Printing a filter writes text that parses back to the same filter, so a
//! filter can travel in a link, a cache key or a log.

use khalid_api::query::{Condition, Filter, Literal, Predicate};
use proptest::prelude::{BoxedStrategy, Just, Strategy, any, prop_assert_eq, prop_oneof, proptest};

#[test]
fn prints_the_canonical_form() {
    let cases = [
        ("tag:news", "tag:'news'"),
        ("tag:-draft+featured:true", "tag:-'draft'+featured:true"),
        ("tag:[seo,links]", "tag:['seo','links']"),
        ("at:>=5,at:<-2", "at:>=5,at:<-2"),
        ("title:~'Don\\'t'", "title:~'Don\\'t'"),
        ("a:1+(b:2,c:3)", "a:1+(b:2,c:3)"),
        ("(a:1+b:2),c:null", "a:1+b:2,c:null"),
        (
            "slug:-~x+slug:~^y+slug:~$z",
            "slug:-~'x'+slug:~^'y'+slug:~$'z'",
        ),
    ];
    for (input, want) in cases {
        let filter: Filter = input.parse().unwrap();
        assert_eq!(filter.to_string(), want, "{input}");
    }
}

#[cfg(test)]
fn literal() -> impl Strategy<Value = Literal> {
    prop_oneof![
        "\\PC{0,12}".prop_map(Literal::Text),
        (0_i64..i64::MAX).prop_map(Literal::Integer),
        any::<bool>().prop_map(Literal::Bool),
        Just(Literal::Null),
    ]
}

#[cfg(test)]
fn predicate() -> impl Strategy<Value = Predicate> {
    let list = || proptest::collection::vec(literal(), 1..4);
    let text = || "\\PC{0,12}";
    prop_oneof![
        literal().prop_map(Predicate::Is),
        literal().prop_map(Predicate::IsNot),
        literal().prop_map(Predicate::Greater),
        literal().prop_map(Predicate::GreaterOrEqual),
        literal().prop_map(Predicate::Less),
        literal().prop_map(Predicate::LessOrEqual),
        list().prop_map(Predicate::In),
        list().prop_map(Predicate::NotIn),
        text().prop_map(Predicate::Contains),
        text().prop_map(Predicate::NotContains),
        text().prop_map(Predicate::StartsWith),
        text().prop_map(Predicate::EndsWith),
    ]
}

#[cfg(test)]
fn filter() -> BoxedStrategy<Filter> {
    let condition = ("[a-z_][a-z0-9_.]{0,8}", predicate())
        .prop_map(|(field, predicate)| Filter::Condition(Condition::new(&field, predicate)));
    condition
        .prop_recursive(4, 24, 3, |inner| {
            let parts = proptest::collection::vec(inner, 2..4);
            prop_oneof![
                parts.clone().prop_map(Filter::And),
                parts.prop_map(Filter::Or)
            ]
        })
        .boxed()
}

proptest! {
    #[test]
    fn printing_then_parsing_gives_the_same_filter(original in filter()) {
        let printed = original.to_string();
        let parsed: Filter = printed.parse().unwrap();
        prop_assert_eq!(parsed, original, "{}", printed);
    }
}
