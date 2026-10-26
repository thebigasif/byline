//! One fixture per schema type, each passing Google's checks (task 1.14,
//! KH-SEO-3). The types the CMS writes are built from the demo site; the
//! types with no block in Phase 1 are JSON-LD files under
//! `tests/fixtures/schema/`, read through the custom field.

#![cfg(test)]

mod common;

use std::time::Duration;

use common::{at, demo_site, internal_links_post, internal_links_post_de, locale, path, url};
use khalid_seo::{
    AggregateRating, CustomSchema, IssueLevel, PageKind, Rating, Recipe, Review, SchemaBlock,
    SchemaGraph, SchemaIssue, SchemaValue, SeoPage, Video, VideoSource, check_graph, page_graph,
};
use serde_json::Value;

fn blocking(graph: &SchemaGraph) -> Vec<String> {
    let issues: Vec<SchemaIssue> = check_graph(graph);
    let blocks = issues.iter().filter(|i| i.level() == IssueLevel::Blocks);
    blocks.map(|i| i.message().what().to_owned()).collect()
}

fn every_block() -> Vec<SchemaBlock> {
    let mut video = Video::new(
        "Internal links in five minutes",
        "A short walk through adding internal links to an older post.",
        url("https://www.example.com/blog/media/internal-links-thumb.jpg"),
        VideoSource::Player(url("https://www.youtube-nocookie.com/embed/aqz-KE-bpKQ")),
    );
    video.duration = Some(Duration::from_secs(312));
    video.uploaded = Some(at(2026, 9, 12, 10));
    let mut review = Review::new(
        "SoftwareApplication",
        "Screaming Frog SEO Spider",
        Rating::new(4.5, 5.0).unwrap(),
        "Asif",
    );
    review.aggregate = Some(AggregateRating::new(Rating::new(4.6, 5.0).unwrap(), 212));
    let mut recipe = Recipe::new(
        "Office flapjacks",
        vec![String::from("250 g rolled oats")],
        vec![String::from("Stir in the oats and press into a lined tin.")],
    );
    recipe.images = vec![url("https://www.example.com/blog/media/flapjacks.jpg")];
    vec![
        SchemaBlock::Video(video),
        SchemaBlock::Review(review),
        SchemaBlock::Recipe(recipe),
    ]
}

#[test]
fn a_post_with_every_block_has_nothing_that_blocks() {
    for mut post in [internal_links_post(), internal_links_post_de()] {
        post.blocks = every_block();
        let graph = page_graph(&demo_site(), &post).unwrap();
        assert_eq!(blocking(&graph), Vec::<String>::new(), "{}", post.title);
    }
}

#[test]
fn every_page_kind_has_nothing_that_blocks() {
    let kinds = [
        PageKind::Home,
        PageKind::Page,
        PageKind::Author,
        PageKind::Tag,
        PageKind::Category,
        PageKind::Collection,
        PageKind::Date,
        PageKind::Search,
        PageKind::GlossaryIndex,
        PageKind::GlossaryTerm,
        PageKind::NotFound,
    ];
    for kind in kinds {
        let mut page = SeoPage::new(kind, locale("en"), path("/asif/"), "Asif");
        page.author = Some(String::from("Asif"));
        let graph = page_graph(&demo_site(), &page).unwrap();
        assert_eq!(blocking(&graph), Vec::<String>::new(), "{kind:?}");
    }
}

#[test]
fn an_author_page_is_a_profile_page_about_its_person() {
    let page = SeoPage::new(
        PageKind::Author,
        locale("en"),
        path("/author/asif/"),
        "Asif",
    );
    let graph = page_graph(&demo_site(), &page).unwrap();
    let profile = graph.nodes().iter().find(|n| n.kind() == "ProfilePage");
    let person = graph.nodes().iter().find(|n| n.kind() == "Person").unwrap();
    let Some(SchemaValue::Reference(main)) = profile.unwrap().get("mainEntity") else {
        panic!("the profile page names no main entity");
    };
    assert_eq!(Some(main), person.id());
    assert_eq!(
        main.as_str(),
        "https://www.example.com/blog/author/asif/#person"
    );
}

#[test]
fn each_type_without_a_block_round_trips_and_raises_nothing() {
    let names = [
        "book",
        "claim-review",
        "course",
        "dataset",
        "event",
        "item-list",
        "podcast-episode",
        "product",
        "software-application",
    ];
    for name in names {
        let file = format!(
            "{}/tests/fixtures/schema/{name}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(&file).unwrap();
        let graph = SchemaGraph::new(CustomSchema::parse(&text).unwrap().nodes().to_vec());
        assert_eq!(check_graph(&graph), Vec::new(), "{name}");
        let mut written: Value = serde_json::from_str(&graph.to_json()).unwrap();
        let mut given: Value = serde_json::from_str(&text).unwrap();
        given.as_object_mut().unwrap().remove("@context");
        assert_eq!(
            written["@graph"].take(),
            Value::Array(vec![given]),
            "{name}"
        );
    }
}
