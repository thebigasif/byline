//! What a Ghost post says about itself beyond its body: its summary, feature
//! image, SEO and share card fields, canonical address and code injection,
//! from Ghost 5 and from older exports.

use std::fs;

use khalid_import::ghost::{PostKind, read_export};

#[cfg(test)]
fn fixture(name: &str) -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    fs::read_to_string(format!("{dir}/fixtures/ghost/{name}")).unwrap()
}

#[test]
fn a_post_keeps_its_summary_image_and_seo_from_posts_meta() {
    let export = read_export(&fixture("ghost-5.json")).unwrap();
    let post = &export.posts[0];
    assert!(post.featured);
    assert_eq!(
        post.excerpt.as_deref(),
        Some("A starter needs food, warmth and time.")
    );
    assert_eq!(
        post.feature_image.as_deref(),
        Some("__GHOST_URL__/content/images/2024/03/starter-jar.jpg")
    );
    let seo = &post.seo;
    assert_eq!(
        seo.meta_title.as_deref(),
        Some("Keep a sourdough starter alive | Northfield")
    );
    assert_eq!(
        seo.meta_description.as_deref(),
        Some("How to feed a sourdough starter so it stays strong.")
    );
    assert_eq!(seo.og_title.as_deref(), Some("Keep your starter alive"));
    assert_eq!(
        seo.og_description.as_deref(),
        Some("Food, warmth and time.")
    );
    assert_eq!(
        seo.og_image.as_deref(),
        Some("__GHOST_URL__/content/images/2024/03/starter-og.jpg")
    );
    assert_eq!(seo.twitter_title.as_deref(), Some("A starter that lives"));
    assert_eq!(seo.twitter_description, None, "an empty field is no field");
    assert_eq!(seo.twitter_image, None);
}

#[test]
fn code_injection_is_noted_and_blank_fields_are_none() {
    let export = read_export(&fixture("ghost-5.json")).unwrap();
    let injected = &export.posts[0].code_injection;
    assert_eq!(
        injected.head.as_deref(),
        Some("<script src=\"https://stats.example.net/a.js\"></script>")
    );
    assert_eq!(injected.foot, None);
    let page = &export.posts[1];
    assert_eq!(
        page.canonical_url.as_deref(),
        Some("https://northfield.example.com/about-us/")
    );
    assert_eq!(page.seo.meta_title, None, "no posts_meta row");
    assert!(!page.featured);
    assert_eq!(
        export.posts[2].excerpt, None,
        "a blank summary is no summary"
    );
}

#[test]
fn an_older_export_keeps_seo_on_the_post_numbers_for_ids_and_flags_for_pages() {
    let text = r#"{"db":[{"data":{"posts":[{"id":12,"slug":"about","page":1,"featured":0,"status":"published","published_at":1393632000000,"meta_title":"About us","og_title":"Old card"},{"id":13,"slug":"rye","page":false,"featured":true,"status":"published","published_at":null}],"posts_meta":[{"post_id":12,"og_title":"New card","meta_title":""}]}}]}"#;
    let export = read_export(text).unwrap();
    let (about, rye) = (&export.posts[0], &export.posts[1]);
    assert_eq!((about.id.as_str(), &about.kind), ("12", &PostKind::Page));
    assert_eq!((rye.id.as_str(), &rye.kind), ("13", &PostKind::Post));
    assert!(!about.featured && rye.featured);
    assert_eq!(
        about.published.map(|at| at.to_string()).as_deref(),
        Some("2014-03-01 00:00:00"),
        "milliseconds since 1970, in UTC"
    );
    assert_eq!(
        about.seo.meta_title.as_deref(),
        Some("About us"),
        "a blank posts_meta field leaves the post's own"
    );
    assert_eq!(
        about.seo.og_title.as_deref(),
        Some("New card"),
        "posts_meta wins"
    );
}
