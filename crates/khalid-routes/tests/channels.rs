//! Reading `routes.toml`: taxonomy prefixes, channels with filters, and the
//! defaults when a site has no file (plan section 4.11, RULES.md BL-FS-3).

use std::fs;
use std::path::Path;

use khalid_core::Slug;
use khalid_routes::{Channel, Routes, RoutesError};

#[cfg(test)]
fn read(text: &str) -> Result<Routes, RoutesError> {
    Routes::from_toml(Path::new("site/routes.toml"), text)
}

/// The line, field and problem of a located error.
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

#[test]
fn a_site_with_no_routes_file_gets_the_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let routes = Routes::load(dir.path()).unwrap();
    assert_eq!(routes, Routes::default());
    assert_eq!(routes.tag_prefix(), "tag");
    assert_eq!(routes.category_prefix(), "category");
    assert_eq!(routes.channels(), []);
}

#[test]
fn a_site_with_a_routes_file_reads_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut path = dir.path().to_path_buf();
    path.push("routes.toml");
    fs::write(&path, "[taxonomies]\ncategory = \"section\"\n").unwrap();
    assert_eq!(
        Routes::load(dir.path()).unwrap().category_prefix(),
        "section"
    );
}

#[test]
fn a_routes_path_that_is_a_folder_cannot_be_read() {
    let dir = tempfile::tempdir().unwrap();
    let mut path = dir.path().to_path_buf();
    path.push("routes.toml");
    fs::create_dir(&path).unwrap();
    let error = Routes::load(dir.path()).unwrap_err();
    assert!(matches!(error, RoutesError::Read { .. }), "{error:?}");
}

#[test]
fn prefixes_and_channels_are_read() {
    let routes = read(
        r#"
[taxonomies]
tag = "topic"

[channels.featured]
path = "/featured/"
filter = { featured = true }

[channels.from-the-founder]
path = "/from-the-founder/"
filter = { author = "asif", collection = "posts" }
"#,
    )
    .unwrap();
    assert_eq!(routes.tag_prefix(), "topic");
    assert_eq!(routes.category_prefix(), "category");
    let names: Vec<&str> = routes.channels().iter().map(Channel::name).collect();
    assert_eq!(names, ["featured", "from-the-founder"]);
    let founder = &routes.channels()[1];
    assert_eq!(founder.path(), "/from-the-founder/");
    let filter = founder.filter();
    assert_eq!(filter.author.as_ref().map(Slug::as_str), Some("asif"));
    assert_eq!(filter.collection.as_deref(), Some("posts"));
    assert_eq!(filter.featured, None);
}

#[test]
fn a_channel_needs_a_filter() {
    let (line, field, problem) =
        located("[channels.everything]\npath = \"/everything/\"\nfilter = {}\n");
    assert_eq!((line, field.as_str()), (3, "filter"));
    assert!(problem.contains("at least one"), "{problem}");
}

#[test]
fn a_channel_path_is_a_plain_folder() {
    let (line, field, _) = located(
        "[channels.featured]\npath = \"/featured/{slug}/\"\nfilter = { featured = true }\n",
    );
    assert_eq!((line, field.as_str()), (2, "path"));
}

#[test]
fn a_filter_with_an_unknown_key_is_refused() {
    let (line, _, problem) =
        located("[channels.featured]\npath = \"/featured/\"\nfilter = { pinned = true }\n");
    assert_eq!(line, 3);
    assert!(problem.contains("pinned"), "{problem}");
}

#[test]
fn a_taxonomy_prefix_is_one_slug() {
    let (line, field, problem) = located("[taxonomies]\ntag = \"/topic/\"\n");
    assert_eq!((line, field.as_str()), (2, "tag"));
    assert!(problem.contains("such as topic"), "{problem}");
}
