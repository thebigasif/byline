//! Reading a `WordPress` site through its public REST API.

use std::path::Path;

use khalid_core::{Config, Environment};
use khalid_import::SlugOrigin;
use khalid_import::wordpress::{ItemKind, ItemStatus, RestAnswers, plan, read_rest};
use khalid_routes::Routes;

const ROOT: &str = r#"{"name":"Northfield &#038; Rye","url":"https://northfield.example.com","home":"https://northfield.example.com"}"#;

const POSTS: &str = r#"[{"id":10,"date":"2024-03-12T09:30:44","slug":"keep-a-starter-alive","status":"publish","type":"post","link":"https://northfield.example.com/2024/03/keep-a-starter-alive/","title":{"rendered":"How to keep a starter alive &#8211; daily"},"content":{"rendered":"<p>Feed it <strong>every day</strong>.</p>"},"excerpt":{"rendered":"<p>A starter needs food.</p>"},"author":2,"sticky":true,"categories":[5],"tags":[7],"yoast_head_json":{"title":"Starters &#8211; Northfield","description":"Feed it daily.","robots":{"index":"noindex","follow":"follow"}}}]"#;

const PAGES: &str = r#"[{"id":11,"date":"2024-01-01T08:00:00","slug":"about","status":"publish","type":"page","link":"https://northfield.example.com/about/","title":{"rendered":"About"},"content":{"rendered":"<p>We bake.</p>"},"author":2,"parent":0}]"#;

#[cfg(test)]
fn answers() -> RestAnswers {
    let mut answers = RestAnswers::default();
    answers.root = ROOT.to_owned();
    answers.posts = vec![POSTS.to_owned(), "[]".to_owned()];
    answers.pages = vec![PAGES.to_owned()];
    answers.users = vec![r#"[{"id":2,"name":"Mara Lind","slug":"mara-lind"}]"#.to_owned()];
    answers.categories =
        vec![r#"[{"id":5,"name":"Sourdough","slug":"sourdough","parent":0}]"#.to_owned()];
    answers.tags = vec![r#"[{"id":7,"name":"Rye","slug":"rye"}]"#.to_owned()];
    answers
}

#[test]
fn posts_pages_people_and_terms_come_back_as_an_export() {
    let export = read_rest(&answers()).unwrap();
    assert_eq!(export.site.title, "Northfield & Rye");
    assert_eq!(
        export.site.address.as_str(),
        "https://northfield.example.com/"
    );
    let post = &export.items[0];
    assert_eq!(
        (post.id, &post.kind, &post.status),
        (10, &ItemKind::Post, &ItemStatus::Publish)
    );
    assert_eq!(post.title, "How to keep a starter alive \u{2013} daily");
    assert_eq!((post.creator.as_str(), post.sticky), ("mara-lind", true));
    assert_eq!(
        (post.categories.as_slice(), post.tags.as_slice()),
        (&["sourdough".to_owned()][..], &["rye".to_owned()][..])
    );
    assert_eq!(
        post.meta("_yoast_wpseo_title"),
        Some("Starters \u{2013} Northfield")
    );
    assert_eq!(post.meta("_yoast_wpseo_meta-robots-noindex"), Some("1"));
    assert_eq!(post.meta("_yoast_wpseo_meta-robots-nofollow"), None);
    assert_eq!(export.items[1].kind, ItemKind::Page);
    assert_eq!(export.authors[0].display_name, "Mara Lind");
}

#[test]
fn the_plan_reads_a_rest_export_as_it_reads_a_file() {
    let toml = "name = \"Northfield Bakery\"\naddress = \"https://northfield.example.com\"\nmount = \"/blog/\"\nmode = \"server\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n";
    let site = Config::from_toml(Path::new("site.toml"), toml, &Environment::empty()).unwrap();
    let plan = plan(
        &read_rest(&answers()).unwrap(),
        None,
        &site,
        &Routes::default(),
    );
    assert!(plan.failed.is_empty(), "{:?}", plan.failed);
    let post = &plan.items[0];
    assert_eq!(
        (post.slug.as_str(), &post.slug_origin),
        ("keep-a-starter-alive", &SlugOrigin::Kept)
    );
    let file = &plan.files[0];
    assert!(file.text.contains("featured: true"), "{}", file.text);
    assert!(
        file.text.contains("Feed it **every day**."),
        "{}",
        file.text
    );
    let report = plan.report();
    assert_eq!((report.posts, report.pages), (1, 1));
}

#[test]
fn an_answer_that_is_not_wordpress_json_is_named() {
    let mut answers = answers();
    answers.tags = vec!["<html>Not found</html>".to_owned()];
    let refused = read_rest(&answers).unwrap_err();
    assert!(
        refused
            .to_string()
            .starts_with("the answer for tags is not what WordPress gives there"),
        "{refused}"
    );
}
