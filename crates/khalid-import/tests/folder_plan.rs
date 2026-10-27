//! Turning a folder of markdown files into Khalid content files.

use std::path::Path;

use khalid_core::{AbsoluteUrl, Config, Environment};
use khalid_import::folder::{Plan, plan, read_folder};
use khalid_import::{Place, SlugOrigin};
use khalid_routes::Routes;

const FILES: [(&str, &str); 6] = [
    (
        "_posts/2024-03-12-why-rye.md",
        "---\ntitle: Why rye\ndate: 2024-03-12 09:30:00 +0100\nexcerpt: A grain that waits\ntags: rye\n---\nRye needs time.\n",
    ),
    (
        "_drafts/spring-flour.md",
        "---\ntitle: Spring flour\n---\nSoon.\n",
    ),
    (
        "content/posts/starters/index.md",
        "+++\ntitle = \"Starters\"\ndate = 2024-05-01\naliases = [\"/blog/old-starters/\"]\n+++\nKeep it warm.\n",
    ),
    ("content/about.md", "---\ntitle: About\n---\nWe bake.\n"),
    (
        "content/posts/why-rye.md",
        "---\ntitle: Why rye, again\ndate: 2024-06-01\n---\nStill rye.\n",
    ),
    (
        "src/content/blog/rye-again.md",
        "---\ntitle: Rye again\ndate: 2024-07-01\n---\nMore rye.\n",
    ),
];

/// The plan of the folder from the site at `old` into a blog mounted at
/// `/blog/` on northfield.example.com.
#[cfg(test)]
fn planned(old: &str) -> Plan {
    let toml = "name = \"Northfield Bakery\"\naddress = \"https://northfield.example.com\"\nmount = \"/blog/\"\nmode = \"server\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n";
    let site = Config::from_toml(Path::new("site.toml"), toml, &Environment::empty()).unwrap();
    let export = read_folder(FILES).unwrap();
    plan(
        &export,
        &AbsoluteUrl::parse(old).unwrap(),
        &site,
        &Routes::default(),
    )
}

#[test]
fn each_file_goes_to_its_place_with_its_slug() {
    let plan = planned("https://rye.example.org");
    let placed: Vec<(&str, &str, Place)> = plan
        .posts
        .iter()
        .map(|post| (post.source.as_str(), post.slug.as_str(), post.place))
        .collect();
    assert_eq!(
        placed,
        [
            ("_drafts/spring-flour.md", "spring-flour", Place::Draft),
            ("_posts/2024-03-12-why-rye.md", "why-rye", Place::Post),
            ("content/about.md", "about", Place::Page),
            ("content/posts/starters/index.md", "starters", Place::Post),
            (
                "content/posts/why-rye.md",
                "content-posts-why-rye-md",
                Place::Post
            ),
            ("src/content/blog/rye-again.md", "rye-again", Place::Post),
        ]
    );
    assert!(matches!(
        plan.posts[4].slug_origin,
        SlugOrigin::Changed { .. }
    ));
    assert!(plan.failed.is_empty(), "{:?}", plan.failed);
}

#[test]
fn a_jekyll_post_keeps_its_date_description_and_names_its_tags() {
    let plan = planned("https://rye.example.org");
    let rye = &plan.files[1];
    assert_eq!(
        rye.path,
        Path::new("content/en/posts/2024-03-12-why-rye.md")
    );
    assert_eq!(
        rye.text,
        "---\ntitle: \"Why rye\"\nslug: \"why-rye\"\ndate: \"2024-03-12\"\ndescription: \"A grain that waits\"\n---\n\nRye needs time.\n"
    );
    assert_eq!(plan.posts[1].left, ["tags"]);
}

#[test]
fn old_addresses_follow_each_generator() {
    let plan = planned("https://rye.example.org");
    let old: Vec<Vec<&str>> = plan
        .posts
        .iter()
        .map(|post| post.old.iter().map(AbsoluteUrl::as_str).collect())
        .collect();
    assert_eq!(
        old,
        [
            vec![],
            vec!["https://rye.example.org/2024/03/12/why-rye.html"],
            vec!["https://rye.example.org/about/"],
            vec![
                "https://rye.example.org/posts/starters/",
                "https://rye.example.org/blog/old-starters/"
            ],
            vec!["https://rye.example.org/posts/why-rye/"],
            vec!["https://rye.example.org/blog/rye-again/"],
        ]
    );
}
