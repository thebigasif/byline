//! Applying a filter to one resource. A field may hold several values,
//! such as a post's tags, and a condition holds when any value passes.

use khalid_api::query::{FieldValues, Filter, Literal, UnknownFieldError};

/// A post as the filter sees it.
struct Post {
    slug: &'static str,
    tags: Vec<&'static str>,
    featured: bool,
    published_at: &'static str,
    reading_time: i64,
    feature_image: Option<&'static str>,
}

impl FieldValues for Post {
    fn values(&self, field: &str) -> Vec<Literal> {
        let text = |value: &str| Literal::Text(value.to_owned());
        match field {
            "slug" => vec![text(self.slug)],
            "tag" => self.tags.iter().map(|tag| text(tag)).collect(),
            "featured" => vec![Literal::Bool(self.featured)],
            "published_at" => vec![text(self.published_at)],
            "reading_time" => vec![Literal::Integer(self.reading_time)],
            "feature_image" => self.feature_image.map(text).into_iter().collect(),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
fn post() -> Post {
    Post {
        slug: "why-internal-links-matter",
        tags: vec!["seo", "links"],
        featured: true,
        published_at: "2026-03-14T09:00:00Z",
        reading_time: 6,
        feature_image: None,
    }
}

#[cfg(test)]
fn holds(filter: &str) -> bool {
    filter.parse::<Filter>().unwrap().matches(&post())
}

#[test]
fn each_operator_holds_or_not() {
    let cases = [
        ("tag:seo", true),
        ("tag:news", false),
        ("tag:-news", true),
        ("tag:-seo", false),
        ("tag:[news,links]", true),
        ("tag:-[news,links]", false),
        ("featured:true", true),
        ("featured:false", false),
        ("published_at:>'2026-01-01'", true),
        ("published_at:<'2026-01-01'", false),
        ("reading_time:>=6", true),
        ("reading_time:<=5", false),
        ("reading_time:>'5'", false),
        ("slug:~'INTERNAL'", true),
        ("slug:-~'internal'", false),
        ("slug:~^'why-'", true),
        ("slug:~$'-matter'", true),
        ("slug:~$'-why'", false),
        ("feature_image:null", true),
        ("feature_image:-null", false),
        ("tag:null", false),
    ];
    for (filter, want) in cases {
        assert_eq!(holds(filter), want, "{filter}");
    }
}

#[test]
fn and_or_and_groups_combine() {
    assert!(holds("tag:seo+featured:true"));
    assert!(!holds("tag:seo+featured:false"));
    assert!(holds("tag:news,featured:true"));
    assert!(!holds("tag:news,featured:false"));
    assert!(holds("tag:news+featured:false,slug:~'links'"));
    assert!(!holds("tag:news+(featured:false,slug:~'links')"));
}

#[test]
fn fields_are_checked_against_what_the_resource_offers() {
    let allowed = ["slug", "tag", "featured"];
    let filter: Filter = "tag:seo+(featured:true,colour:red)".parse().unwrap();
    let error: UnknownFieldError = filter.check_fields(&allowed).unwrap_err();
    assert_eq!(error.field(), "colour");
    assert_eq!(
        error.to_string(),
        "the filter uses colour, which is not a field here"
    );
    let known: Filter = "tag:seo+featured:true".parse().unwrap();
    assert_eq!(known.check_fields(&allowed), Ok(()));
}
