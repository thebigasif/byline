//! The list parameters every list endpoint shares (RULES.md BL-API-5):
//! defaults, each parameter read, and each mistake named.

use khalid_api::query::{Direction, Format, Limit, ListQuery, RawListQuery};
use proptest::prelude::{prop_assert_eq, proptest};

#[cfg(test)]
fn parse(pairs: &[(&str, &str)]) -> Result<ListQuery, String> {
    let mut raw = RawListQuery::default();
    for (name, value) in pairs {
        let slot = match *name {
            "filter" => &mut raw.filter,
            "include" => &mut raw.include,
            "fields" => &mut raw.fields,
            "order" => &mut raw.order,
            "formats" => &mut raw.formats,
            "limit" => &mut raw.limit,
            "page" => &mut raw.page,
            // Other parameters, such as `key`, are not list parameters.
            _ => continue,
        };
        *slot = Some((*value).to_owned());
    }
    ListQuery::try_from(&raw).map_err(|error| error.to_string())
}

#[test]
fn nothing_given_means_the_defaults() {
    let query = parse(&[("key", "ignored")]).unwrap();
    assert_eq!(query.filter(), None);
    assert_eq!(query.formats(), [Format::Html]);
    assert_eq!(query.limit(), Limit::Count(15));
    assert_eq!(query.page(), 1);
}

#[test]
fn every_parameter_is_read() {
    let query = parse(&[
        ("filter", "tag:news"),
        ("formats", "plaintext,html,plaintext"),
        ("limit", "all"),
        ("page", "3"),
    ])
    .unwrap();
    let filter = query.filter().map(ToString::to_string);
    assert_eq!(filter.as_deref(), Some("tag:'news'"));
    assert_eq!(query.formats(), [Format::Plaintext, Format::Html]);
    assert_eq!(query.limit(), Limit::All);
    assert_eq!(query.page(), 3);
}

#[test]
fn include_fields_and_order_are_read() {
    let query = parse(&[
        ("include", "authors,tags,authors"),
        ("fields", "title, slug"),
        ("order", "published_at DESC, title"),
    ])
    .unwrap();
    assert_eq!(query.include(), ["authors", "tags"]);
    assert_eq!(query.fields(), ["title", "slug"]);
    let order: Vec<(&str, Direction)> = query
        .order()
        .iter()
        .map(|key| (key.field(), key.direction()))
        .collect();
    let want = [
        ("published_at", Direction::Descending),
        ("title", Direction::Ascending),
    ];
    assert_eq!(order, want);
    let none = parse(&[]).unwrap();
    assert_eq!(
        (none.include(), none.fields(), none.order()),
        (&[][..], &[][..], &[][..])
    );
}

#[test]
fn name_and_order_mistakes_name_the_parameter() {
    let many = ["title"; 51].join(",");
    let keys = ["title"; 6].join(",");
    let cases = [
        (
            ("include", "authors,Tags!"),
            "the include parameter has Tags!, which is not a field name",
        ),
        (("fields", ""), "the fields parameter has an empty name"),
        (
            ("fields", "title,,slug"),
            "the fields parameter has an empty name",
        ),
        (
            ("fields", many.as_str()),
            "the fields parameter has more than 50 names",
        ),
        (
            ("order", "title sideways"),
            "the order parameter has sideways, which is not asc or desc",
        ),
        (
            ("order", "title asc desc"),
            "the order parameter has title asc desc, which is not a field and a direction",
        ),
        (
            ("order", "9lives"),
            "the order parameter has 9lives, which is not a field name",
        ),
        (
            ("order", keys.as_str()),
            "the order parameter has more than 5 names",
        ),
    ];
    for ((name, value), want) in cases {
        assert_eq!(parse(&[(name, value)]).unwrap_err(), want, "{name}={value}");
    }
}

#[test]
fn mistakes_name_the_parameter() {
    let limit = "the limit parameter must be a whole number from 1 to 100, or all";
    let page = "the page parameter must be a whole number from 1 to 1000000";
    let cases = [
        (
            ("filter", "tag:"),
            "the filter needs a value at character 5",
        ),
        (("limit", "0"), limit),
        (("limit", "101"), limit),
        (("limit", "ten"), limit),
        (("page", "0"), page),
        (("page", "-2"), page),
        (
            ("formats", "html,pdf"),
            "the formats parameter has pdf, which is not html, markdown or plaintext",
        ),
    ];
    for ((name, value), want) in cases {
        assert_eq!(parse(&[(name, value)]).unwrap_err(), want, "{name}={value}");
    }
}

#[test]
fn a_bad_filter_keeps_its_cause() {
    let mut raw = RawListQuery::default();
    raw.filter = Some("tag:".to_owned());
    let error = ListQuery::try_from(&raw).unwrap_err();
    assert_eq!(error.parameter(), "filter");
    assert!(std::error::Error::source(&error).is_some());
}

proptest! {
    #[test]
    fn any_limit_and_page_in_range_is_read(limit in 1_u32..=100, page in 1_u32..=1_000_000) {
        let query = parse(&[("limit", &limit.to_string()), ("page", &page.to_string())]).unwrap();
        prop_assert_eq!(query.limit(), Limit::Count(limit));
        prop_assert_eq!(query.page(), page);
    }

    #[test]
    fn no_value_makes_a_parameter_panic(value in "\\PC{0,64}") {
        for name in ["filter", "include", "fields", "order", "formats", "limit", "page"] {
            let _ = parse(&[(name, &value)]);
        }
    }
}
