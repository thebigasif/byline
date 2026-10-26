//! The HTML of the recipe block, and its structured data.

use std::time::Duration;

use super::{Image, Renderer, attribute};
use crate::{RecipeBlock, StructuredBlock};

impl Renderer<'_> {
    /// Writes a recipe: its name, photo, line about it, what it makes and
    /// how long it takes, then the ingredients and the numbered method.
    pub(super) fn recipe(&mut self, recipe: &RecipeBlock) {
        let name = attribute(&recipe.name);
        self.push(&[
            "<section class=\"recipe\">\n<p class=\"recipe-name\">",
            &name,
            "</p>\n",
        ]);
        if let Some(image) = &recipe.image {
            self.push(&[
                "<img src=\"",
                &attribute(image),
                "\" alt=\"",
                &name,
                "\" loading=\"lazy\" decoding=\"async\" />\n",
            ]);
            self.facts.images.push(Image {
                src: image.clone(),
                alt: Some(recipe.name.clone()),
            });
        }
        if let Some(description) = &recipe.description {
            self.push(&[
                "<p class=\"recipe-description\">",
                &attribute(description),
                "</p>\n",
            ]);
        }
        self.facts_line(recipe);
        self.list("ul", "recipe-ingredients", &recipe.ingredients);
        self.list("ol", "recipe-method", &recipe.steps);
        self.html.push_str("</section>\n");
        self.facts
            .structured
            .push(StructuredBlock::Recipe(recipe.clone()));
    }

    /// Writes what a recipe makes and its times, as `h:mm` in `<time>`.
    fn facts_line(&mut self, recipe: &RecipeBlock) {
        let servings = recipe.servings.as_deref().map(attribute);
        let mut parts: Vec<String> = servings
            .map(|servings| format!("<span class=\"recipe-servings\">{servings}</span>"))
            .into_iter()
            .collect();
        for (class, time) in [("recipe-prep", recipe.prep), ("recipe-cook", recipe.cook)] {
            if let Some(time) = time {
                parts.push(time_element(class, time));
            }
        }
        if !parts.is_empty() {
            self.push(&["<p class=\"recipe-facts\">", &parts.join(" "), "</p>\n"]);
        }
    }

    /// Writes `items` as a `tag` list with `class`.
    fn list(&mut self, tag: &str, class: &str, items: &[String]) {
        self.push(&["<", tag, " class=\"", class, "\">\n"]);
        for item in items {
            self.push(&["<li>", &attribute(item), "</li>\n"]);
        }
        self.push(&["</", tag, ">\n"]);
    }
}

/// A `<time>` element for `time`, shown as `h:mm` with its ISO 8601 duration.
fn time_element(class: &str, time: Duration) -> String {
    let minutes = time.as_secs() / 60;
    let (hours, minutes) = (minutes / 60, minutes % 60);
    format!("<time class=\"{class}\" datetime=\"PT{hours}H{minutes}M\">{hours}:{minutes:02}</time>")
}
