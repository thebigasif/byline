//! Recipe blocks: ingredients and a method, nested per ADR 0012, that give
//! structured data (task 1.7, KH-CON-5).

use std::time::Duration;

use khalid_content::{BlockKind, RecipeBlock, StructuredBlock, parse_blocks, print_blocks, render};
use khalid_core::{Locale, SlugMode};

const RECIPE: &str = "\
::::recipe{name=\"Oat bars\" description=\"Chewy bars for a long walk.\" image=\"/media/oat-bars.jpg\" servings=\"12 bars\" prep=\"15:00\" cook=\"30:00\" category=\"Snack\" cuisine=\"British\" calories=\"180\"}
:::ingredients
- 250 g rolled oats
- 125 g butter
:::

:::method
1. Melt the butter and stir in the oats.
2. Bake at 180 C for 30 minutes.
:::
::::
";

#[cfg(test)]
fn text(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[cfg(test)]
fn oat_bars() -> RecipeBlock {
    RecipeBlock {
        name: "Oat bars".to_owned(),
        description: Some("Chewy bars for a long walk.".to_owned()),
        image: Some("/media/oat-bars.jpg".to_owned()),
        servings: Some("12 bars".to_owned()),
        prep: Some(Duration::from_mins(15)),
        cook: Some(Duration::from_mins(30)),
        category: Some("Snack".to_owned()),
        cuisine: Some("British".to_owned()),
        calories: Some(180),
        ingredients: text(&["250 g rolled oats", "125 g butter"]),
        steps: text(&[
            "Melt the butter and stir in the oats.",
            "Bake at 180 C for 30 minutes.",
        ]),
    }
}

#[cfg(test)]
fn error(markdown: &str) -> String {
    parse_blocks(markdown).unwrap_err().to_string()
}

#[test]
fn a_recipe_round_trips() {
    let blocks = parse_blocks(RECIPE).unwrap();
    assert_eq!(print_blocks(&blocks), RECIPE);
    assert_eq!(blocks[0].kind, BlockKind::Recipe(oat_bars()));
}

#[test]
fn renders_a_recipe_and_its_structured_data() {
    let page = render(RECIPE, &Locale::try_new("en").unwrap(), SlugMode::Ascii).unwrap();
    assert_eq!(page.structured, [StructuredBlock::Recipe(oat_bars())]);
    insta::assert_snapshot!(page.html);
}

#[test]
fn a_recipe_needs_ingredients_and_a_method() {
    assert_eq!(
        error("::::recipe{name=\"Toast\"}\n:::method\n1. Toast the bread.\n:::\n::::\n"),
        "line 1: a recipe needs a :::ingredients list and a :::method list, each closed with :::"
    );
}

#[test]
fn ingredients_are_a_list() {
    assert_eq!(
        error("::::recipe{name=\"Toast\"}\n:::ingredients\nBread\n:::\n::::\n"),
        "line 3: each line of a recipe's list starts with - or a number, such as - 250 g oats"
    );
}
