//! Turning a Ghost export into Khalid content files.

use std::fs;
use std::path::Path;

use khalid_core::{AbsoluteUrl, Config, Environment};
use khalid_import::ghost::{Export, LeftReason, Plan, PostError, Visibility, plan, read_export};
use khalid_import::{Place, SlugOrigin};
use khalid_routes::Routes;

/// The plan of `export` from the Ghost site at `northfield.example.com`
/// into a blog mounted at `/blog/` on the same host.
#[cfg(test)]
fn planned(export: &Export) -> Plan {
    let toml = "name = \"Northfield Bakery\"\naddress = \"https://northfield.example.com\"\nmount = \"/blog/\"\nmode = \"server\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n";
    let site = Config::from_toml(Path::new("site.toml"), toml, &Environment::empty()).unwrap();
    let old = AbsoluteUrl::parse("https://northfield.example.com").unwrap();
    plan(export, &old, &site, &Routes::default())
}

#[cfg(test)]
fn fixture() -> Export {
    let dir = env!("CARGO_MANIFEST_DIR");
    read_export(&fs::read_to_string(format!("{dir}/fixtures/ghost/ghost-5.json")).unwrap()).unwrap()
}

#[test]
fn a_published_post_becomes_a_dated_file_that_reads_back() {
    let plan = planned(&fixture());
    let file = &plan.files[0];
    assert_eq!(
        file.path,
        Path::new("content/en/posts/2024-03-12-keep-a-starter-alive.md")
    );
    assert_eq!(
        file.text,
        "---\n\
         title: \"How to keep a starter alive\"\n\
         slug: \"keep-a-starter-alive\"\n\
         date: \"2024-03-12\"\n\
         author: \"mara\"\n\
         description: \"How to feed a sourdough starter so it stays strong.\"\n\
         featured: true\n\
         ---\n\
         \n\
         **Feed it every day.**\n"
    );
    let post = &plan.posts[0];
    assert_eq!(post.slug_origin, SlugOrigin::Kept);
    assert_eq!(post.address.as_deref(), Some("/blog/keep-a-starter-alive/"));
}

#[test]
fn pages_drafts_scheduled_and_members_posts_find_their_places() {
    let plan = planned(&fixture());
    let placed: Vec<(Place, &Path)> = plan
        .posts
        .iter()
        .map(|post| (post.place, post.path.as_path()))
        .collect();
    assert_eq!(
        placed[1],
        (Place::Page, Path::new("content/en/pages/about.md"))
    );
    let drafts =
        ["rye-slowly", "spring-flour", "members-loaf"].map(|slug| format!("drafts/en/{slug}.md"));
    for (n, draft) in drafts.iter().enumerate() {
        assert_eq!(placed[n + 2], (Place::Draft, Path::new(draft)));
    }
    assert_eq!(plan.posts[1].address.as_deref(), Some("/blog/about/"));
    assert_eq!(
        plan.posts[4].address, None,
        "a members' post is not published"
    );
    assert!(plan.files[1].text.ends_with("---\n\nWe bake rye.\n"));
    assert!(plan.failed.is_empty() && plan.left_behind.is_empty());
}

#[test]
fn newsletters_unpublished_and_members_pages_stay_behind_with_the_reason() {
    let text = r#"{"db":[{"data":{"posts":[{"id":"a1","type":"post","status":"sent"},{"id":"a2","type":"page","status":"draft"},{"id":"a3","type":"page","status":"published","visibility":"paid"},{"id":"a4","type":"post","status":"archived"}]}}]}"#;
    let plan = planned(&read_export(text).unwrap());
    let reasons: Vec<&LeftReason> = plan.left_behind.iter().map(|post| &post.reason).collect();
    let archived = LeftReason::Status("archived".to_owned());
    assert_eq!(
        reasons,
        [
            &LeftReason::Newsletter,
            &LeftReason::UnpublishedPage,
            &LeftReason::MembersPage(Visibility::Paid),
            &archived,
        ]
    );
    assert_eq!(plan.left_behind[3].id, "a4");
    let paths: Vec<&Path> = plan.files.iter().map(|file| file.path.as_path()).collect();
    assert_eq!(
        paths,
        [Path::new("import/ghost-redirects.csv")],
        "no content file"
    );
}

#[test]
fn a_post_that_cannot_come_in_is_listed_and_ghost_urls_are_filled_in() {
    let text = r#"{"db":[{"data":{"posts":[{"id":"b1","slug":"no-date","status":"published"},{"id":"b2","status":"draft","lexical":"{\"root\":"},{"id":"b3","slug":"Über Rye!","status":"draft","html":"<p>See <a href=\"__GHOST_URL__/rye/\">rye</a>.</p>"}]}}]}"#;
    let plan = planned(&read_export(text).unwrap());
    let failed: Vec<(&str, String)> = plan
        .failed
        .iter()
        .map(|post| (post.id.as_str(), post.error.to_string()))
        .collect();
    assert_eq!(
        failed[0],
        ("b1", "the post was published but has no date".to_owned())
    );
    assert_eq!(failed[1], ("b2", "its body could not be read".to_owned()));
    assert!(matches!(plan.failed[1].error, PostError::Body(_)));
    let was = "Über Rye!".to_owned();
    assert_eq!(plan.posts[0].slug.as_str(), "uber-rye");
    assert_eq!(plan.posts[0].slug_origin, SlugOrigin::Changed { was });
    assert!(
        plan.files[0]
            .text
            .ends_with("See [rye](https://northfield.example.com/rye/).\n")
    );
}
