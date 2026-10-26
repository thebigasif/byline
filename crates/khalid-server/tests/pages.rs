//! Page paths are clean, and each address holds at most one page.

use khalid_server::{MediaType, Page, PagePath, Pages, ParsePagePathError, ServerError};

#[test]
fn a_page_path_is_a_clean_absolute_path() {
    for good in ["/", "/blog/", "/de/blog/über-seo/", "/robots.txt"] {
        let path = PagePath::try_new(good).unwrap();
        assert_eq!(path.as_str(), good);
    }
    for bad in ["", "blog/", "robots.txt"] {
        let error = PagePath::try_new(bad).unwrap_err();
        assert!(
            matches!(error, ParsePagePathError::Relative { .. }),
            "{bad:?}"
        );
    }
    for bad in ["//", "//evil.example", "/blog//", "/blog/../", "/./"] {
        let error = PagePath::try_new(bad).unwrap_err();
        assert!(
            matches!(error, ParsePagePathError::Folder { .. }),
            "{bad:?}"
        );
    }
    for (bad, refused) in [
        ("/blog?x", '?'),
        ("/blog#top", '#'),
        ("/a\\b", '\\'),
        ("/a\nb", '\n'),
    ] {
        let error = PagePath::try_new(bad).unwrap_err();
        assert!(
            matches!(error, ParsePagePathError::Character { character, .. } if character == refused),
            "{bad:?}"
        );
    }
}

#[test]
fn holds_each_page_by_its_path() {
    let path = PagePath::try_new("/blog/what-a-sitemap-does/").unwrap();
    let page = Page::new(path, MediaType::Html, "<h1>What a sitemap does</h1>");
    let pages = Pages::new(vec![page.clone()], "<h1>Page not found</h1>").unwrap();
    let held = pages.get("/blog/what-a-sitemap-does/").unwrap();
    assert_eq!(held.path(), page.path());
    assert_eq!(held.body(), page.body());
    assert_eq!(pages.get("/blog/what-a-sitemap-does"), None);
    assert_eq!(pages.not_found().as_ref(), b"<h1>Page not found</h1>");
}

#[test]
fn refuses_two_pages_at_one_address() {
    let page =
        |body: &'static str| Page::new(PagePath::try_new("/blog/").unwrap(), MediaType::Html, body);
    let error = Pages::new(vec![page("first"), page("second")], "").unwrap_err();
    assert!(
        matches!(&error, ServerError::DuplicatePage { path } if path.as_str() == "/blog/"),
        "{error:?}"
    );
}

#[test]
fn each_media_type_names_its_charset() {
    assert_eq!(MediaType::Html.as_header(), "text/html; charset=utf-8");
    assert_eq!(MediaType::Xml.as_header(), "application/xml; charset=utf-8");
    assert_eq!(
        MediaType::PlainText.as_header(),
        "text/plain; charset=utf-8"
    );
    assert_eq!(MediaType::Json.as_header(), "application/json");
    assert_eq!(MediaType::Css.as_header(), "text/css; charset=utf-8");
}
