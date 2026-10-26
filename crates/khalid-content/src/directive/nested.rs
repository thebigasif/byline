//! Blocks that hold other blocks, written with more colons on the outer one:
//! `::::faq` around `:::question{...}` (ADR 0012).

use super::{Attributes, clock, opening, read};
use crate::{BlockError, BlockKind, HowToBlock, Question, RecipeBlock, Step};

/// One inner block: its name, attributes and body's markdown.
struct Child {
    name: String,
    attributes: Attributes,
    text: String,
}

/// Reads an FAQ opened on `line`: one `:::question{text="..."}` per
/// question, the answer as its body.
pub(super) fn faq(attributes: &Attributes, body: &[&str]) -> Result<BlockKind, BlockError> {
    attributes.only("faq", &[], "nothing")?;
    let questions = children(attributes.line, body, &["question"], FAQ_SHAPE)?
        .into_iter()
        .map(|child| {
            child.attributes.only("question", &["text"], "text")?;
            Ok(Question {
                text: child
                    .attributes
                    .need("text", "a question needs text, the question")?,
                answer: child.text,
            })
        })
        .collect::<Result<Vec<_>, BlockError>>()?;
    if questions.is_empty() {
        return Err(BlockError::Value {
            line: attributes.line,
            problem: "an FAQ needs at least one question, written :::question{text=\"...\"}, its answer, then :::",
        });
    }
    Ok(BlockKind::Faq { questions })
}

/// Reads a how-to: its name and time, and one `:::step{name="..."}` per
/// step, what to do as its body.
pub(super) fn how_to(attributes: &Attributes, body: &[&str]) -> Result<BlockKind, BlockError> {
    attributes.only("howto", &["name", "time"], "name and time")?;
    let name = attributes.need("name", "a how-to needs name, what it helps a reader do")?;
    let time = attributes
        .get("time")
        .map(|text| {
            clock(&text).ok_or(BlockError::Value {
                line: attributes.line,
                problem: "a how-to's time is written as minutes:seconds, such as 10:00, or hours:minutes:seconds",
            })
        })
        .transpose()?;
    let steps = children(attributes.line, body, &["step"], HOW_TO_SHAPE)?
        .into_iter()
        .map(|child| {
            child.attributes.only("step", &["name"], "name")?;
            Ok(Step {
                name: child
                    .attributes
                    .need("name", "a step needs name, a few words for it")?,
                text: child.text,
            })
        })
        .collect::<Result<Vec<_>, BlockError>>()?;
    if steps.is_empty() {
        return Err(BlockError::Value {
            line: attributes.line,
            problem: HOW_TO_SHAPE_EMPTY,
        });
    }
    Ok(BlockKind::HowTo(HowToBlock { name, time, steps }))
}

/// Reads a recipe: its facts as attributes, then a `:::ingredients` list
/// and a `:::method` list.
pub(super) fn recipe(attributes: &Attributes, body: &[&str]) -> Result<BlockKind, BlockError> {
    attributes.only(
        "recipe",
        &RECIPE_ATTRIBUTES,
        "name, description, image, servings, prep, cook, category, cuisine and calories",
    )?;
    let name = attributes.need("name", "a recipe needs name, the dish")?;
    let (mut ingredients, mut steps) = (None, None);
    for child in children(attributes.line, body, &RECIPE_PARTS, RECIPE_SHAPE)? {
        let (block, slot) = if child.name == "ingredients" {
            ("ingredients", &mut ingredients)
        } else {
            ("method", &mut steps)
        };
        child.attributes.only(block, &[], "nothing")?;
        if slot.is_none() {
            *slot = Some(list(child.attributes.line, &child.text)?);
        }
    }
    let (Some(ingredients), Some(steps)) = (ingredients, steps) else {
        return Err(BlockError::Value {
            line: attributes.line,
            problem: RECIPE_SHAPE,
        });
    };
    let time = "a recipe's prep and cook times are written as minutes:seconds, such as 15:00";
    Ok(BlockKind::Recipe(RecipeBlock {
        name,
        description: attributes.get("description"),
        image: attributes
            .get("image")
            .map(|image| attributes.media("recipe", "image", image))
            .transpose()?,
        servings: attributes.get("servings"),
        prep: attributes.parsed("prep", clock, time)?,
        cook: attributes.parsed("cook", clock, time)?,
        category: attributes.get("category"),
        cuisine: attributes.get("cuisine"),
        calories: attributes.parsed(
            "calories",
            |text| text.parse().ok(),
            "a recipe's calories are a whole number per serving, such as 180",
        )?,
        ingredients,
        steps,
    }))
}

const RECIPE_ATTRIBUTES: [&str; 9] = [
    "name",
    "description",
    "image",
    "servings",
    "prep",
    "cook",
    "category",
    "cuisine",
    "calories",
];
const RECIPE_PARTS: [&str; 2] = ["ingredients", "method"];

/// The items of a markdown list, one per line, in `text` from the inner
/// block opened on `line`.
fn list(line: usize, text: &str) -> Result<Vec<String>, BlockError> {
    let mut items = Vec::new();
    for (number, item) in (line.saturating_add(1)..).zip(text.lines()) {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let numbered = item
            .split_once(". ")
            .filter(|(digits, _)| digits.parse::<u32>().is_ok())
            .map(|(_, rest)| rest);
        let Some(rest) = item.strip_prefix("- ").or(numbered) else {
            return Err(BlockError::Value {
                line: number,
                problem: "each line of a recipe's list starts with - or a number, such as - 250 g oats",
            });
        };
        items.push(rest.trim().to_owned());
    }
    Ok(items)
}

const RECIPE_SHAPE: &str =
    "a recipe needs a :::ingredients list and a :::method list, each closed with :::";
const FAQ_SHAPE: &str =
    "an FAQ holds only questions, each written :::question{text=\"...\"}, its answer, then :::";
const HOW_TO_SHAPE: &str =
    "a how-to holds only steps, each written :::step{name=\"...\"}, what to do, then :::";
const HOW_TO_SHAPE_EMPTY: &str =
    "a how-to needs at least one step, written :::step{name=\"...\"}, what to do, then :::";

/// Reads the `:::child{...}` blocks in the `body` of a block opened on
/// `line`. Blank lines between them are fine; any other line is refused with
/// `shape`, which says how the block is written.
fn children(
    line: usize,
    body: &[&str],
    names: &[&str],
    shape: &'static str,
) -> Result<Vec<Child>, BlockError> {
    let mut found = Vec::new();
    let mut index = 0;
    while let Some(text) = body.get(index) {
        let number = line.saturating_add(1).saturating_add(index);
        if text.trim().is_empty() {
            index = index.saturating_add(1);
            continue;
        }
        let rest = match opening(text) {
            Some((":::", rest)) if names.contains(&rest.split('{').next().unwrap_or_default()) => {
                rest
            }
            _ => {
                return Err(BlockError::Value {
                    line: number,
                    problem: shape,
                });
            }
        };
        let Some(close) = body
            .iter()
            .skip(index.saturating_add(1))
            .position(|line| line.trim_end() == ":::")
            .map(|offset| index.saturating_add(1).saturating_add(offset))
        else {
            return Err(BlockError::Unclosed {
                line: number,
                name: rest.split('{').next().unwrap_or_default().to_owned(),
            });
        };
        let (name, attributes) = read(number, rest)?;
        let text = body
            .get(index.saturating_add(1)..close)
            .unwrap_or_default()
            .join("\n");
        found.push(Child {
            name: name.to_owned(),
            attributes,
            text,
        });
        index = close.saturating_add(1);
    }
    Ok(found)
}
