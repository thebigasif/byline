//! Reading a Ghost JSON export: posts, pages, their status and their bodies.

use khalid_import::ghost::PostStatus::{Draft, Published, Scheduled, Sent};
use khalid_import::ghost::{Body, ExportError, PostKind, PostStatus, Visibility, read_export};

/// A Ghost 5 export, as Settings, Import/Export writes it, with `posts`.
#[cfg(test)]
fn export(posts: &str) -> String {
    format!(
        r#"{{"db":[{{"meta":{{"exported_on":1730000000000,"version":"5.96.0"}},"data":{{"posts":[{posts}],"settings":[{{"key":"title","value":"Northfield Bakery"}},{{"key":"codeinjection_head","value":""}}]}}}}]}}"#
    )
}

#[test]
fn reads_every_post_and_page_in_file_order_with_its_status() {
    let text = export(concat!(
        r#"{"id":"6650a1f2c3d4e5f6a7b8c901","uuid":"0e3c1a2b","title":"How to keep a starter alive","slug":"keep-a-starter-alive","type":"post","status":"published","visibility":"public","published_at":"2024-03-12T09:30:00.000Z"},"#,
        r#"{"id":"6650a1f2c3d4e5f6a7b8c902","title":"About the bakery","slug":"about","type":"page","status":"published","visibility":"public"},"#,
        r#"{"id":"6650a1f2c3d4e5f6a7b8c903","title":"Rye, slowly","slug":"rye-slowly","type":"post","status":"draft","published_at":null},"#,
        r#"{"id":"6650a1f2c3d4e5f6a7b8c904","slug":"spring-flour","type":"post","status":"scheduled"},"#,
        r#"{"id":"6650a1f2c3d4e5f6a7b8c905","slug":"members-loaf","type":"post","status":"published","visibility":"paid"},"#,
        r#"{"id":"6650a1f2c3d4e5f6a7b8c906","slug":"spring-letter","type":"post","status":"sent","visibility":"members"}"#,
    ));
    let export = read_export(&text).unwrap();
    assert_eq!(export.title.as_deref(), Some("Northfield Bakery"));
    assert_eq!(export.version.as_deref(), Some("5.96.0"));
    let posts = &export.posts;
    let slugs: Vec<&str> = posts.iter().map(|post| post.slug.as_str()).collect();
    assert_eq!(slugs[..2], ["keep-a-starter-alive", "about"]);
    assert_eq!(
        (&posts[0].kind, &posts[1].kind),
        (&PostKind::Post, &PostKind::Page)
    );
    let status: Vec<PostStatus> = posts.iter().map(|post| post.status.clone()).collect();
    assert_eq!(
        status,
        [Published, Published, Draft, Scheduled, Published, Sent]
    );
    let seen: Vec<&Visibility> = posts.iter().map(|post| &post.visibility).collect();
    assert_eq!(
        seen[3..],
        [&Visibility::Public, &Visibility::Paid, &Visibility::Members]
    );
    assert_eq!(posts[0].id, "6650a1f2c3d4e5f6a7b8c901");
    assert_eq!(posts[0].title, "How to keep a starter alive");
    assert_eq!(
        posts[0].published.map(|at| at.to_string()).as_deref(),
        Some("2024-03-12 09:30:00"),
        "dates stay in UTC"
    );
    assert_eq!(posts[2].published, None, "a draft has no date");
}

#[test]
fn the_body_is_lexical_then_mobiledoc_then_html() {
    let text = export(concat!(
        r#"{"id":"a1","slug":"a","lexical":"{\"root\":{}}","mobiledoc":"{}","html":"<p>A</p>"},"#,
        r#"{"id":"a2","slug":"b","lexical":null,"mobiledoc":"{\"sections\":[]}","html":"<p>B</p>"},"#,
        r#"{"id":"a3","slug":"c","lexical":"","html":"<p>Not finished.</p>"},"#,
        r#"{"id":"a4","slug":"d","html":"  "}"#,
    ));
    let export = read_export(&text).unwrap();
    let bodies: Vec<&Body> = export.posts.iter().map(|post| &post.body).collect();
    assert_eq!(
        bodies,
        [
            &Body::Lexical("{\"root\":{}}".to_owned()),
            &Body::Mobiledoc("{\"sections\":[]}".to_owned()),
            &Body::Html("<p>Not finished.</p>".to_owned()),
            &Body::Empty,
        ]
    );
}

#[test]
fn a_file_that_is_not_a_ghost_export_is_refused_with_the_reason() {
    let broken = read_export("{\"db\": [").unwrap_err();
    assert!(
        matches!(broken, ExportError::NotJson { line: 1, .. }),
        "{broken:?}"
    );
    let other = read_export("{\"posts\": []}").unwrap_err();
    assert!(matches!(other, ExportError::NotGhost), "{other:?}");
    let empty = read_export("{\"db\": []}").unwrap_err();
    assert!(matches!(empty, ExportError::NotGhost), "{empty:?}");
    let date = export(r#"{"id":"a1","slug":"rye","published_at":"last spring"}"#);
    assert_eq!(
        read_export(&date).unwrap_err().to_string(),
        "post a1: published_at holds \"last spring\", which is not what Ghost writes there"
    );
}
