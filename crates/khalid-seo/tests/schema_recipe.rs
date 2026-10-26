//! The recipe block (task 1.14, KH-SEO-3).

#![cfg(test)]

mod common;

use std::time::Duration;

use common::{demo_site, internal_links_post, internal_links_post_de, url};
use khalid_seo::{Recipe, SchemaBlock, SchemaGraph, SchemaValue, page_graph};

fn flapjacks() -> Recipe {
    let mut recipe = Recipe::new(
        "Office flapjacks",
        vec![
            String::from("250 g rolled oats"),
            String::from("125 g butter"),
            String::from("125 g golden syrup"),
        ],
        vec![
            String::from("Melt the butter and syrup in a pan."),
            String::from("Stir in the oats and press into a lined tin."),
            String::from("Bake at 180 °C for 20 minutes."),
        ],
    );
    recipe.description = String::from("The flapjacks we bake for launch days.");
    recipe.images = vec![url("https://www.example.com/blog/media/flapjacks.jpg")];
    recipe.prep_time = Some(Duration::from_mins(10));
    recipe.cook_time = Some(Duration::from_mins(20));
    recipe.servings = Some(String::from("12 bars"));
    recipe.category = Some(String::from("Snack"));
    recipe.cuisine = Some(String::from("British"));
    recipe.calories = Some(210);
    recipe
}

#[test]
fn a_recipe_block_adds_a_recipe() {
    let mut post = internal_links_post();
    post.blocks = vec![SchemaBlock::Recipe(flapjacks())];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let node = graph.nodes().last().unwrap().clone();
    assert_eq!(node.kind(), "Recipe");
    insta::assert_snapshot!(SchemaGraph::new(vec![node]).to_json());
}

#[test]
fn the_total_time_is_prep_plus_cook_and_absent_without_both() {
    let mut post = internal_links_post_de();
    let mut quick = flapjacks();
    quick.cook_time = None;
    post.blocks = vec![SchemaBlock::Recipe(quick)];
    let graph = page_graph(&demo_site(), &post).unwrap();
    let node = graph.nodes().last().unwrap();
    assert_eq!(node.get("totalTime"), None);
    assert!(node.get("prepTime").is_some());
    let Some(SchemaValue::Node(author)) = node.get("author") else {
        panic!("the recipe names no author");
    };
    assert_eq!(author.get("name"), Some(&SchemaValue::text("Asif")));
}
