//! Turning a `WordPress` export into Khalid content files.

use std::fs;
use std::path::Path;

use khalid_core::{AbsoluteUrl, Config, Environment};
use khalid_import::wordpress::{Held, ItemError, ItemStatus, SlugOrigin, plan, read_wxr};

#[cfg(test)]
fn site() -> Config {
    let toml = "name = \"Northfield Bakery\"\naddress = \"https://northfield.example.com\"\nmount = \"/blog/\"\nmode = \"server\"\ndefault_language = \"en\"\nlanguages = [\"en\"]\n";
    Config::from_toml(Path::new("site.toml"), toml, &Environment::empty()).unwrap()
}

#[cfg(test)]
fn basic() -> khalid_import::wordpress::Export {
    let dir = env!("CARGO_MANIFEST_DIR");
    read_wxr(&fs::read_to_string(format!("{dir}/fixtures/wordpress/basic.xml")).unwrap()).unwrap()
}

#[test]
fn a_published_post_becomes_a_dated_file_with_its_front_matter() {
    let plan = plan(&basic(), &site());
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
         author: \"mara-lind\"\n\
         category: \"sourdough\"\n\
         description: \"A starter needs food, warmth and time.\"\n\
         featured: true\n\
         ---\n\
         \n\
         Feed it **every day**.\n"
    );
}

#[test]
fn the_post_keeps_its_slug_and_lists_what_front_matter_cannot_hold() {
    let plan = plan(&basic(), &site());
    let post = &plan.items[0];
    assert_eq!((post.id, post.slug.as_str()), (10, "keep-a-starter-alive"));
    assert_eq!(post.slug_origin, SlugOrigin::Kept);
    let image = "https://northfield.example.com/wp-content/uploads/2024/03/starter-jar.jpg";
    assert_eq!(
        post.held,
        [
            Held::Tags(vec!["rye".to_owned()]),
            Held::FeatureImage(AbsoluteUrl::parse(image).unwrap()),
        ]
    );
    assert!(plan.failed.is_empty(), "{:?}", plan.failed);
}

#[test]
fn search_titles_and_odd_text_are_carried() {
    let text = fs::read_to_string(format!(
        "{}/fixtures/wordpress/basic.xml",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
    .replace(
        "<wp:postmeta><wp:meta_key><![CDATA[_thumbnail_id]]>",
        "<wp:postmeta><wp:meta_key><![CDATA[_yoast_wpseo_title]]></wp:meta_key><wp:meta_value><![CDATA[Starters &amp; you]]></wp:meta_value></wp:postmeta>\n<wp:postmeta><wp:meta_key><![CDATA[_thumbnail_id]]>",
    )
    .replace("How to keep a starter alive", "Bread &amp; \"butter\": a starter");
    let plan = plan(&read_wxr(&text).unwrap(), &site());
    assert!(
        plan.files[0]
            .text
            .contains("title: \"Bread & \\\"butter\\\": a starter\"\n"),
        "{}",
        plan.files[0].text
    );
    assert_eq!(
        plan.items[0].held[0],
        Held::SearchTitle("Starters & you".to_owned())
    );
}

#[test]
fn a_published_post_with_no_date_is_listed_not_written() {
    let mut export = basic();
    export.items[0].published = None;
    let plan = plan(&export, &site());
    let posts = Path::new("content/en/posts");
    assert!(!plan.files.iter().any(|file| file.path.starts_with(posts)));
    assert_eq!(plan.failed.len(), 1);
    assert_eq!(plan.failed[0].id, 10);
    assert!(matches!(plan.failed[0].error, ItemError::NoDate));
}

#[test]
fn pages_drafts_authors_and_categories_get_their_files() {
    let plan = plan(&basic(), &site());
    let paths: Vec<&Path> = plan.files.iter().map(|file| file.path.as_path()).collect();
    assert_eq!(
        paths,
        [
            "content/en/posts/2024-03-12-keep-a-starter-alive.md",
            "content/en/pages/about.md",
            "drafts/en/rye-a-first-loaf.md",
            "content/en/authors/mara-lind.md",
            "content/en/authors/tom-becker.md",
            "categories.toml",
        ]
        .map(Path::new)
    );
    let text = |n: usize| plan.files[n].text.as_str();
    assert_eq!(
        text(1),
        "---\ntitle: \"About the bakery\"\nslug: \"about\"\n---\n\nWe bake in Northfield since 1998.\n"
    );
    assert!(
        !text(2).contains("date:"),
        "a draft has no date: {}",
        text(2)
    );
    assert!(text(2).contains("author: \"tom-becker\"\ncategory: \"baking\"\n"));
    assert_eq!(plan.items[2].slug_origin, SlugOrigin::Made);
    assert_eq!(
        text(3),
        "---\nname: \"Mara Lind\"\nslug: \"mara-lind\"\n---\n\n"
    );
    assert_eq!(
        text(5),
        "# Categories from the WordPress export of Northfield Bakery Blog.\n\n\
         [[categories]]\nslug = \"baking\"\nen = \"Baking\"\n\n\
         [[categories]]\nslug = \"sourdough\"\nen = \"Sourdough & starters\"\n"
    );
}

#[test]
fn every_planned_file_reads_back_with_the_site_s_types() {
    let plan = plan(&basic(), &site());
    for file in &plan.files {
        let path = file.path.to_string_lossy();
        if path.contains("/authors/") {
            let yaml = file
                .text
                .trim_start_matches("---\n")
                .split("---\n")
                .next()
                .unwrap();
            let author: khalid_content::AuthorFront = serde_saphyr::from_str(yaml).unwrap();
            assert!(author.name.is_some(), "{path}");
        } else if path == "categories.toml" {
            let table: toml::Table = file.text.parse().unwrap();
            assert_eq!(table["categories"].as_array().unwrap().len(), 2);
        } else {
            khalid_content::parse_post(&file.path, &file.text).unwrap();
        }
    }
}

#[test]
fn a_scheduled_post_waits_in_drafts_with_its_time() {
    let mut export = basic();
    export.items[0].status = ItemStatus::Future;
    let plan = plan(&export, &site());
    assert_eq!(
        plan.files[0].path,
        Path::new("drafts/en/keep-a-starter-alive.md")
    );
    let due = export.items[0].published.unwrap();
    assert_eq!(plan.items[0].held.last(), Some(&Held::Scheduled(due)));
}
