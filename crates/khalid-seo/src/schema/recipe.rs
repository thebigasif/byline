//! The recipe block: `Recipe`.

use std::time::Duration;

use khalid_core::AbsoluteUrl;

use crate::schema::node_id;
use crate::{SchemaNode, SchemaValue, SeoError, SeoPage};

/// A recipe block.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Recipe {
    /// The dish.
    pub name: String,
    /// One line about it.
    pub description: String,
    /// Photos of the dish.
    pub images: Vec<AbsoluteUrl>,
    /// Each ingredient with its amount, such as `250 g rolled oats`.
    pub ingredients: Vec<String>,
    /// The steps, in order.
    pub steps: Vec<String>,
    /// Time to prepare.
    pub prep_time: Option<Duration>,
    /// Time to cook.
    pub cook_time: Option<Duration>,
    /// What it makes, such as `12 bars` or `4 servings`.
    pub servings: Option<String>,
    /// The kind of dish, such as `Dessert`.
    pub category: Option<String>,
    /// Where it comes from, such as `British`.
    pub cuisine: Option<String>,
    /// Calories per serving.
    pub calories: Option<u32>,
}

impl Recipe {
    /// A recipe with its name, ingredients and steps.
    pub fn new(name: &str, ingredients: Vec<String>, steps: Vec<String>) -> Self {
        Self {
            name: name.to_owned(),
            description: String::new(),
            images: Vec::new(),
            ingredients,
            steps,
            prep_time: None,
            cook_time: None,
            servings: None,
            category: None,
            cuisine: None,
            calories: None,
        }
    }
}

fn texts(list: &[String]) -> SchemaValue {
    SchemaValue::List(list.iter().map(|text| SchemaValue::text(text)).collect())
}

/// The `Recipe` at `<page>#recipe-<number>`. The author and date come from
/// the post; the total time is the prep and cook times added, when both are set.
pub(crate) fn recipe_node(
    recipe: &Recipe,
    page: &SeoPage,
    url: &AbsoluteUrl,
    number: usize,
) -> Result<SchemaNode, SeoError> {
    let steps = recipe
        .steps
        .iter()
        .map(|step| {
            let node = SchemaNode::new("HowToStep").with("text", SchemaValue::text(step));
            SchemaValue::Node(Box::new(node))
        })
        .collect();
    let mut node = SchemaNode::new("Recipe")
        .with_id(node_id(url, &format!("recipe-{number}"))?)
        .with("name", SchemaValue::text(&recipe.name))
        .with("recipeIngredient", texts(&recipe.ingredients))
        .with("recipeInstructions", SchemaValue::List(steps));
    if !recipe.description.is_empty() {
        node.set("description", SchemaValue::text(&recipe.description));
    }
    if !recipe.images.is_empty() {
        let images = recipe
            .images
            .iter()
            .cloned()
            .map(SchemaValue::Url)
            .collect();
        node.set("image", SchemaValue::List(images));
    }
    if let Some(author) = &page.author {
        let person = SchemaNode::new("Person").with("name", SchemaValue::text(author));
        node.set("author", SchemaValue::Node(Box::new(person)));
    }
    if let Some(published) = page.published {
        node.set("datePublished", SchemaValue::DateTime(published));
    }
    let times = [
        ("prepTime", recipe.prep_time),
        ("cookTime", recipe.cook_time),
    ];
    for (property, time) in times {
        if let Some(time) = time {
            node.set(property, SchemaValue::Duration(time));
        }
    }
    if let (Some(prep), Some(cook)) = (recipe.prep_time, recipe.cook_time) {
        node.set(
            "totalTime",
            SchemaValue::Duration(prep.saturating_add(cook)),
        );
    }
    let labels = [
        ("recipeYield", &recipe.servings),
        ("recipeCategory", &recipe.category),
        ("recipeCuisine", &recipe.cuisine),
    ];
    for (property, label) in labels {
        if let Some(label) = label {
            node.set(property, SchemaValue::text(label));
        }
    }
    if let Some(calories) = recipe.calories {
        let nutrition = SchemaNode::new("NutritionInformation").with(
            "calories",
            SchemaValue::Text(format!("{calories} calories")),
        );
        node.set("nutrition", SchemaValue::Node(Box::new(nutrition)));
    }
    Ok(node)
}
