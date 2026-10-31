//! Photo and GIF search on a running `khalid serve`: on or off from
//! `[media]` in `site.toml`, with keys from the environment only (G50,
//! KH-EDT-1, decision E6). Nothing here reaches Unsplash or GIPHY.

mod common;

use std::fs;

const SEARCH: &str = "/api/admin/v1/media/search/";

#[test]
fn photo_search_turned_on_without_its_key_names_the_variable() {
    let site = common::demo_copy();
    let mut settings = site.path().to_path_buf();
    settings.push("site.toml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(&settings, format!("{text}\n[media]\nunsplash = true\n")).unwrap();
    let key = common::new_key(site.path(), &["--name", "Maria", "--scope", "media:write"]);
    let server = common::start(site.path(), &key);
    let (status, on) = server.admin("GET", SEARCH, None);
    assert_eq!(status, 200, "{on}");
    assert_eq!(on["unsplash"], true, "{on}");
    assert_eq!(on["gifs"], false, "{on}");
    let words = format!("{SEARCH}?source=unsplash&words=lake");
    let (status, problem) = server.admin("GET", &words, None);
    assert_eq!(status, 503, "{problem}");
    let fix = problem["fix"].as_str().unwrap();
    assert!(fix.contains("KHALID_UNSPLASH_ACCESS_KEY"), "{problem}");
    let gifs = format!("{SEARCH}?source=gifs&words=dance");
    let (status, problem) = server.admin("GET", &gifs, None);
    assert_eq!(status, 503, "{problem}");
    assert!(problem["fix"].as_str().unwrap().contains("gifs = true"));
}

#[test]
fn a_blog_that_never_turned_them_on_offers_neither() {
    let site = common::demo_copy();
    let key = common::new_key(site.path(), &["--name", "Maria", "--scope", "media:write"]);
    let server = common::start(site.path(), &key);
    let (status, on) = server.admin("GET", SEARCH, None);
    assert_eq!(status, 200, "{on}");
    assert_eq!(
        (&on["unsplash"], &on["gifs"]),
        (&false.into(), &false.into())
    );
    assert!(
        server.answers("/blog/", 200),
        "readers see the blog as ever"
    );
}
