//! Page paths and the addresses built from them (task 1.13).

use khalid_seo::{SeoError, UrlPath};

#[test]
fn paths_that_could_change_meaning_are_refused() {
    for bad in [
        "",
        "blog/",
        "/blog//post/",
        "/blog/../admin/",
        "/./post/",
        "/a b/",
        "/post?id=1",
        "/post#top",
        "/back\\slash/",
    ] {
        let error = UrlPath::try_new(bad).unwrap_err();
        assert!(matches!(error, SeoError::Path { .. }), "{bad:?}: {error:?}");
    }
}

#[test]
fn clean_paths_in_any_script_are_accepted_as_written() {
    for good in ["/", "/blog/", "/de/über-uns/", "/tag/seo", "/ja/検索/"] {
        assert_eq!(UrlPath::try_new(good).unwrap().as_str(), good);
    }
}

#[test]
fn the_error_names_the_path_and_the_problem() {
    let error = UrlPath::try_new("/blog/../admin/").unwrap_err();
    assert_eq!(
        error.to_string(),
        r#"the path "/blog/../admin/" has a . or .. segment"#
    );
}
