//! Reading a folder of markdown files from Hugo, Jekyll, Astro or Eleventy.

use chrono::NaiveDate;
use khalid_import::folder::{FileError, FolderError, read_folder};

const JEKYLL: &str = "---\nlayout: post\ntitle: \"Why rye\"\ndate: 2024-03-12 09:30:00 +0100\ntags: rye baking\nexcerpt: A grain that waits\npublished: false\n---\n\nRye needs time.\n";

const HUGO: &str = "+++\ntitle = \"Spring flour\"\ndate = 2024-04-02T08:00:00Z\nslug = \"spring\"\ntags = [\"flour\"]\naliases = [\"/old/spring/\"]\nurl = \"/notes/spring/\"\n+++\n\nMill it fresh.\n";

const ASTRO: &str = "---\ntitle: Starters\npubDate: 2024-05-01\ndescription: Feed it daily\ndraft: true\n---\nKeep it warm.\n";

#[test]
fn yaml_and_toml_front_matter_give_the_same_fields() {
    let export = read_folder([
        ("_posts/2024-03-12-why-rye.md", JEKYLL),
        ("content/posts/spring.md", HUGO),
        ("src/content/blog/starters.mdx", ASTRO),
        ("src/content/blog/starters.md", ASTRO),
        ("assets/loaf.jpg", "not text"),
    ])
    .unwrap();
    let [jekyll, hugo, astro] = &export.files[..] else {
        panic!("{:?}", export.files);
    };
    assert_eq!(jekyll.path, "_posts/2024-03-12-why-rye.md");
    assert_eq!(jekyll.front.title.as_deref(), Some("Why rye"));
    assert_eq!(jekyll.front.date, NaiveDate::from_ymd_opt(2024, 3, 12));
    assert_eq!(jekyll.front.tags, ["rye", "baking"]);
    assert_eq!(
        jekyll.front.description.as_deref(),
        Some("A grain that waits")
    );
    assert!(jekyll.front.draft);
    assert_eq!(jekyll.body, "Rye needs time.\n");

    assert_eq!(hugo.front.slug.as_deref(), Some("spring"));
    assert_eq!(hugo.front.date, NaiveDate::from_ymd_opt(2024, 4, 2));
    assert_eq!(hugo.front.aliases, ["/old/spring/"]);
    assert_eq!(hugo.front.permalink.as_deref(), Some("/notes/spring/"));
    assert!(!hugo.front.draft);
    assert_eq!(hugo.body, "Mill it fresh.\n");

    assert_eq!(astro.front.date, NaiveDate::from_ymd_opt(2024, 5, 1));
    assert!(astro.front.draft);
}

#[test]
fn files_that_cannot_be_read_are_listed_with_why() {
    let export = read_folder([
        ("a.md", "---\ntitle: [unclosed\n---\nText.\n"),
        ("b.md", "---\ntitle: Late\ndate: someday\n---\nText.\n"),
        ("c.mdx", ASTRO),
        ("d.md", "No front matter, still a post.\n"),
    ])
    .unwrap();
    let unread: Vec<(&str, &FileError)> = export
        .unread
        .iter()
        .map(|file| (file.path.as_str(), &file.error))
        .collect();
    assert!(
        matches!(unread[0], ("a.md", FileError::FrontMatter(_))),
        "{unread:?}"
    );
    assert_eq!(
        unread[1..],
        [
            ("b.md", &FileError::Date("someday".to_owned())),
            ("c.mdx", &FileError::Mdx)
        ]
    );
    assert_eq!(export.files[0].body, "No front matter, still a post.\n");
}

#[test]
fn a_folder_with_no_markdown_is_refused() {
    let refused = read_folder([("index.html", "<p>Hi</p>")]).unwrap_err();
    assert_eq!(refused, FolderError::NoPosts);
}
