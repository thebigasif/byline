//! The people and tags of a Ghost export, and which of them each post has.

use std::fs;

use khalid_import::ghost::read_export;

#[cfg(test)]
fn fixture(name: &str) -> String {
    let dir = env!("CARGO_MANIFEST_DIR");
    fs::read_to_string(format!("{dir}/fixtures/ghost/{name}")).unwrap()
}

#[test]
fn every_author_keeps_their_name_slug_bio_and_profiles() {
    let export = read_export(&fixture("ghost-5.json")).unwrap();
    let mara = &export.authors[0];
    assert_eq!(
        (mara.id.as_str(), mara.name.as_str(), mara.slug.as_str()),
        ("1", "Mara Lind", "mara")
    );
    assert_eq!(mara.bio.as_deref(), Some("Mara bakes the rye."));
    assert_eq!(
        mara.website.as_deref(),
        Some("https://maralind.example.org")
    );
    assert_eq!(mara.twitter.as_deref(), Some("@maralind"));
    assert_eq!(mara.facebook.as_deref(), Some("maralind.bakes"));
    let jonas = &export.authors[1];
    assert_eq!((jonas.slug.as_str(), jonas.bio.as_ref()), ("jonas", None));
}

#[test]
fn tags_are_read_and_internal_ones_marked() {
    let export = read_export(&fixture("ghost-5.json")).unwrap();
    let tags: Vec<(&str, &str, bool)> = export
        .tags
        .iter()
        .map(|tag| (tag.slug.as_str(), tag.name.as_str(), tag.internal))
        .collect();
    assert_eq!(
        tags,
        [
            ("sourdough", "Sourdough", false),
            ("rye", "Rye", false),
            ("hash-newsletter", "#newsletter", true),
        ]
    );
    assert_eq!(
        export.tags[0].description.as_deref(),
        Some("Wild yeast, slowly.")
    );
}

#[test]
fn a_post_lists_its_authors_and_tags_in_their_order() {
    let export = read_export(&fixture("ghost-5.json")).unwrap();
    let post = &export.posts[0];
    assert_eq!(post.authors, ["1", "6650a1f2c3d4e5f6a7b8f002"]);
    assert_eq!(
        post.tags,
        [
            "6650a1f2c3d4e5f6a7b8a001",
            "6650a1f2c3d4e5f6a7b8a002",
            "6650a1f2c3d4e5f6a7b8a003"
        ]
    );
    assert_eq!(export.posts[1].authors, ["1"]);
    assert!(export.posts[2].authors.is_empty() && export.posts[2].tags.is_empty());
}

#[test]
fn an_older_post_names_its_one_author_on_itself() {
    let text = r#"{"db":[{"data":{"posts":[{"id":1,"slug":"rye","author_id":7}],"users":[{"id":7,"name":"Mara Lind","slug":"mara"}],"tags":[{"id":3,"name":"Rye","slug":"rye","visibility":null}],"posts_tags":[{"post_id":1,"tag_id":3}]}}]}"#;
    let export = read_export(text).unwrap();
    assert_eq!(export.posts[0].authors, ["7"]);
    assert_eq!(export.posts[0].tags, ["3"]);
    assert_eq!(export.authors[0].id, "7");
    assert!(!export.tags[0].internal);
}
