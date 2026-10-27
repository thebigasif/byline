//! Turning a Medium export into Khalid content files.

use std::fs;
use std::path::Path;

use khalid_core::{Config, Environment};
use khalid_import::medium::{Export, Plan, plan, read_export};
use khalid_import::{Place, SlugOrigin, WriteError};
use khalid_routes::Routes;

/// A post file as Medium's export writes it, published at `time` with
/// the address `canonical`, or a draft when `time` is empty.
#[cfg(test)]
fn post(title: &str, subtitle: &str, time: &str, canonical: &str) -> String {
    let footer = if time.is_empty() {
        String::new()
    } else {
        format!(
            r#"<p>By <a href="https://medium.com/@mara" class="p-author h-card">Mara Lind</a> on <time class="dt-published" datetime="{time}">then</time>.</p><p><a href="{canonical}" class="p-canonical">Canonical link</a></p>"#
        )
    };
    format!(
        r#"<html><body><article class="h-entry"><header><h1 class="p-name">{title}</h1></header><section data-field="subtitle" class="p-summary">{subtitle}</section><section data-field="body" class="e-content"><section class="section"><div class="section-inner"><h3 class="graf graf--h3 graf--title">{title}</h3><p class="graf graf--p">Rye needs time.</p></div></section></section><footer>{footer}</footer></article></body></html>"#
    )
}

/// The plan of `export` into a blog mounted at `/blog/`.
#[cfg(test)]
fn planned(export: &Export) -> Plan {
    let toml = "name = \"Northfield Bakery\"\naddress = \"https://northfield.example.com\"\nmount = \"/blog/\"\nmode = \"server\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n";
    let site = Config::from_toml(Path::new("site.toml"), toml, &Environment::empty()).unwrap();
    plan(export, &site, &Routes::default())
}

#[cfg(test)]
fn export() -> Export {
    let rye = post(
        "How to bake rye bread",
        "A wetter dough and a longer rest",
        "2024-03-12T09:30:44.123Z",
        "https://medium.com/@mara/how-to-bake-rye-bread-3f1a8c4e2b9d",
    );
    let again = post(
        "How to bake rye bread, again",
        "",
        "2024-05-02T08:00:00.000Z",
        "https://bakery.example.org/how-to-bake-rye-bread-0a1b2c3d4e5f",
    );
    let draft = post("Spring flour", "", "", "");
    read_export([
        ("2024-03-12_How-to-bake-rye-bread-3f1a8c4e2b9d.html", rye),
        (
            "2024-05-02_How-to-bake-rye-bread-again-0a1b2c3d4e5f.html",
            again,
        ),
        ("draft_Spring-flour-9e8d7c6b5a41.html", draft),
        ("notes.txt", String::new()),
    ])
    .unwrap()
}

#[test]
fn a_published_post_becomes_a_dated_file_that_reads_back() {
    let plan = planned(&export());
    let file = &plan.files[0];
    assert_eq!(
        file.path,
        Path::new("content/en/posts/2024-03-12-how-to-bake-rye-bread.md")
    );
    assert_eq!(
        file.text,
        "---\n\
         title: \"How to bake rye bread\"\n\
         slug: \"how-to-bake-rye-bread\"\n\
         date: \"2024-03-12\"\n\
         author: \"mara\"\n\
         description: \"A wetter dough and a longer rest\"\n\
         ---\n\
         \n\
         Rye needs time.\n"
    );
    let post = &plan.posts[0];
    assert_eq!(
        post.slug_origin,
        SlugOrigin::Kept,
        "Medium's id is not part of the slug"
    );
    assert_eq!(
        post.address.as_deref(),
        Some("/blog/how-to-bake-rye-bread/")
    );
    assert_eq!(post.place, Place::Post);
}

#[test]
fn a_taken_slug_keeps_medium_s_whole_slug_and_a_draft_gets_one() {
    let plan = planned(&export());
    let found: Vec<(&str, &SlugOrigin, Place, &Path)> = plan
        .posts
        .iter()
        .map(|post| {
            let path = post.path.as_path();
            (post.slug.as_str(), &post.slug_origin, post.place, path)
        })
        .collect();
    assert_eq!(
        found[1..],
        [
            (
                "how-to-bake-rye-bread-0a1b2c3d4e5f",
                &SlugOrigin::Kept,
                Place::Post,
                Path::new("content/en/posts/2024-05-02-how-to-bake-rye-bread-0a1b2c3d4e5f.md"),
            ),
            (
                "spring-flour",
                &SlugOrigin::Made,
                Place::Draft,
                Path::new("drafts/en/spring-flour.md"),
            ),
        ]
    );
    assert_eq!(plan.posts[2].address, None, "a draft is not published");
    assert!(plan.failed.is_empty(), "{:?}", plan.failed);
    let unread: Vec<&str> = plan.unread.iter().map(|file| file.file.as_str()).collect();
    assert_eq!(unread, ["notes.txt"], "the report lists what was not read");
}

#[test]
fn the_plan_writes_every_file_and_never_over_one() {
    let plan = planned(&export());
    let site = tempfile::tempdir().unwrap();
    let written = plan.write(site.path()).unwrap();
    assert_eq!(written.len(), plan.files.len());
    let mut draft = site.path().to_owned();
    draft.push("drafts/en/spring-flour.md");
    assert!(
        fs::read_to_string(&draft)
            .unwrap()
            .contains("title: \"Spring flour\"")
    );
    let again = plan.write(site.path()).unwrap_err();
    assert!(
        matches!(&again, WriteError::Exists { paths } if paths.len() == 5),
        "{again:?}"
    );
}

#[test]
fn two_drafts_with_one_title_get_two_files() {
    let draft = post("Spring flour", "", "", "");
    let export = read_export([
        ("draft_Spring-flour-9e8d7c6b5a41.html", draft.as_str()),
        ("draft_Spring-flour-1c2d3e4f5a6b.html", draft.as_str()),
    ])
    .unwrap();
    let plan = planned(&export);
    let paths: Vec<&Path> = plan.posts.iter().map(|post| post.path.as_path()).collect();
    assert_eq!(
        paths,
        [
            Path::new("drafts/en/spring-flour.md"),
            Path::new("drafts/en/spring-flour-9e8d7c6b5a41.md"),
        ],
        "Medium's id tells them apart"
    );
}
