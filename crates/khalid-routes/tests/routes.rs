//! Reading `routes.toml`: collections and their permalink patterns, with the
//! line and field of every mistake (plan section 4.11, RULES.md BL-FS-3).

use std::path::Path;

use khalid_routes::{Routes, RoutesError};

#[cfg(test)]
fn read(text: &str) -> Result<Routes, RoutesError> {
    Routes::from_toml(Path::new("site/routes.toml"), text)
}

/// The line, field and problem of an error, for one assertion per case.
#[cfg(test)]
fn located(text: &str) -> (usize, String, String) {
    match read(text).unwrap_err() {
        RoutesError::Parse {
            line,
            field,
            source,
            ..
        } => (line, field, source.message().to_owned()),
        RoutesError::Field {
            line,
            field,
            problem,
            ..
        } => (line, field, problem),
        other => panic!("not a located error: {other:?}"),
    }
}

#[cfg(test)]
fn permalink<'a>(routes: &'a Routes, collection: &str) -> &'a str {
    routes.collection(collection).unwrap().permalink().as_str()
}

#[test]
fn an_empty_file_gives_the_defaults() {
    let routes = read("").unwrap();
    assert_eq!(routes, Routes::default());
    assert_eq!(permalink(&routes, "posts"), "/{slug}/");
}

#[test]
fn collections_are_read_and_the_default_stays() {
    let routes = read(
        "[collections.news]\npermalink = \"/news/{year}/{month}/{slug}/\"\n\n\
         [collections.guides]\npermalink = \"/{collection}/{slug}/\"\n",
    )
    .unwrap();
    assert_eq!(permalink(&routes, "news"), "/news/{year}/{month}/{slug}/");
    assert_eq!(permalink(&routes, "guides"), "/{collection}/{slug}/");
    assert_eq!(permalink(&routes, "posts"), "/{slug}/");
    assert!(routes.collection("glossary").is_none());
}

#[test]
fn a_collection_marked_news_is_news_and_others_are_not() {
    let routes = read(
        "[collections.news]\npermalink = \"/news/{year}/{slug}/\"\nnews = true\n\n\
         [collections.guides]\npermalink = \"/guides/{slug}/\"\nnews = false\n",
    )
    .unwrap();
    let news = |name: &str| routes.collection(name).unwrap().is_news();
    assert!(news("news"));
    assert!(!news("guides"));
    assert!(!news("posts"));
}

#[test]
fn news_takes_true_or_false_only() {
    let (line, field, _) =
        located("[collections.news]\npermalink = \"/news/{slug}/\"\nnews = \"yes\"\n");
    assert_eq!((line, field.as_str()), (3, "news"));
}

#[test]
fn a_permalink_must_hold_the_slug() {
    let (line, field, problem) = located("[collections.news]\npermalink = \"/news/{year}/\"\n");
    assert_eq!((line, field.as_str()), (2, "permalink"));
    assert!(problem.contains("{slug}"), "{problem}");
}

#[test]
fn a_permalink_with_an_unknown_placeholder_is_refused() {
    let text = "[collections.posts]\npermalink = \"/{slug}/\"\n\n\
                [collections.news]\npermalink = \"/news/{week}/{slug}/\"\n";
    let (line, field, problem) = located(text);
    assert_eq!((line, field.as_str()), (5, "permalink"), "the second one");
    assert!(problem.contains("{week}"), "{problem}");
}

#[test]
fn a_permalink_must_start_and_end_with_a_slash() {
    for pattern in ["news/{slug}/", "/news/{slug}", "/news//{slug}/"] {
        let text = format!("[collections.news]\npermalink = \"{pattern}\"\n");
        let (line, field, problem) = located(&text);
        assert_eq!((line, field.as_str()), (2, "permalink"), "{pattern}");
        assert!(problem.contains("start and end with /"), "{problem}");
    }
}

#[test]
fn a_placeholder_used_twice_is_refused() {
    let (_, _, problem) = located("[collections.news]\npermalink = \"/{slug}/{slug}/\"\n");
    assert!(problem.contains("twice"), "{problem}");
}

#[test]
fn an_unclosed_placeholder_is_refused() {
    let (_, _, problem) = located("[collections.news]\npermalink = \"/news/{slug/\"\n");
    assert!(problem.contains("never closed"), "{problem}");
}

#[test]
fn capitals_are_refused() {
    let (_, _, problem) = located("[collections.news]\npermalink = \"/News/{slug}/\"\n");
    assert!(problem.contains("`N` is not allowed"), "{problem}");
}

#[test]
fn an_unknown_field_names_its_line() {
    let (line, field, problem) =
        located("[collections.news]\npermalink = \"/news/{slug}/\"\nlayout = \"wide\"\n");
    assert_eq!((line, field.as_str()), (3, "layout"));
    assert!(problem.contains("unknown field"), "{problem}");
}

#[test]
fn broken_toml_names_its_line() {
    let (line, _, _) = located("[collections.news]\npermalink = \"/news/{slug}/\n");
    assert_eq!(line, 2);
}
