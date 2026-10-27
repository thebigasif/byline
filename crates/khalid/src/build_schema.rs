//! The schema a post's own content writes (plan section 5.9): its FAQ,
//! how-to, review, recipe and video blocks as their schema.org types, its
//! videos for the video sitemap, and the entities it links to as
//! `mentions`.

use khalid_content::{HowToBlock, Link, RecipeBlock, ReviewBlock, StructuredBlock, VideoBlock};
use khalid_core::{AbsoluteUrl, Config, UserMessage};
use khalid_seo::{
    Entity, EntityRole, FaqItem, HowTo, HowToStep, Rating, Recipe, Review, SchemaBlock, SeoPage,
    Video, VideoSource,
};

use crate::build_message;

/// Hosts whose pages name one entity each, so a link to one is a
/// `mentions` with that page as its `sameAs`.
const ENTITY_HOSTS: [&str; 2] = ["wikipedia.org", "wikidata.org"];

/// Fills `seo`, the page of a post at `path` such as `/blog/slug/`, with
/// the schema of its body's `structured` blocks and `links`. A review with no author of its own
/// is credited to `author`, the post's author.
///
/// # Errors
///
/// A [`UserMessage`] when a block's address cannot be made a full address
/// on the site, or a review's score is outside its scale.
pub(crate) fn fill(
    seo: &mut SeoPage,
    config: &Config,
    (path, author): (&str, &str),
    (structured, links): (&[StructuredBlock], &[Link]),
) -> Result<(), UserMessage> {
    let full = |src: &str| full_address(config, path, src);
    for block in structured {
        let schema = match block {
            StructuredBlock::Faq(questions) => {
                let items = questions.iter().map(|q| FaqItem::new(&q.text, &q.answer));
                SchemaBlock::Faq(items.collect())
            }
            StructuredBlock::HowTo(how_to) => SchemaBlock::HowTo(how_to_of(how_to)),
            StructuredBlock::Review(review) => SchemaBlock::Review(review_of(review, author)?),
            StructuredBlock::Recipe(recipe) => SchemaBlock::Recipe(recipe_of(recipe, full)?),
            StructuredBlock::Video(video) => {
                let mut video = video_of(video, full)?;
                video.uploaded = seo.published;
                seo.videos.push(video.clone());
                SchemaBlock::Video(video)
            }
            // Products and businesses have their own schema in a later part.
            _ => continue,
        };
        seo.blocks.push(schema);
    }
    seo.entities = links.iter().filter_map(entity).collect();
    Ok(())
}

/// The how-to block as `HowTo`.
fn how_to_of(block: &HowToBlock) -> HowTo {
    let steps = block.steps.iter().map(|s| HowToStep::new(&s.name, &s.text));
    let mut how_to = HowTo::new(&block.name, steps.collect());
    how_to.total_time = block.time;
    how_to
}

/// The review block as `Review`, by `author` when it names nobody.
fn review_of(block: &ReviewBlock, author: &str) -> Result<Review, UserMessage> {
    let number = |text: &str| text.parse::<f64>().unwrap_or(f64::NAN);
    let rating = Rating::new(number(&block.rating), number(&block.best))
        .map_err(|error| build_message::seo(&error))?;
    let by = block.author.as_deref().unwrap_or(author);
    let mut review = Review::new(&block.item_type, &block.item, rating, by);
    review.body.clone_from(&block.body);
    Ok(review)
}

/// The recipe block as `Recipe`, its photo made a full address by `full`.
fn recipe_of(
    block: &RecipeBlock,
    full: impl Fn(&str) -> Result<AbsoluteUrl, UserMessage>,
) -> Result<Recipe, UserMessage> {
    let mut recipe = Recipe::new(&block.name, block.ingredients.clone(), block.steps.clone());
    recipe.description = block.description.clone().unwrap_or_default();
    recipe.images = block
        .image
        .as_deref()
        .map(full)
        .transpose()?
        .into_iter()
        .collect();
    recipe.prep_time = block.prep;
    recipe.cook_time = block.cook;
    recipe.servings.clone_from(&block.servings);
    recipe.category.clone_from(&block.category);
    recipe.cuisine.clone_from(&block.cuisine);
    recipe.calories = block.calories;
    Ok(recipe)
}

/// The video block as `Video`, its file and poster made full addresses by
/// `full`.
fn video_of(
    block: &VideoBlock,
    full: impl Fn(&str) -> Result<AbsoluteUrl, UserMessage>,
) -> Result<Video, UserMessage> {
    let description = block.description.as_deref().unwrap_or(&block.title);
    let source = VideoSource::File(full(&block.src)?);
    let mut video = Video::new(&block.title, description, full(&block.poster)?, source);
    video.duration = block.duration;
    Ok(video)
}

/// `link` as an entity the post mentions, when it points at a page that
/// names one, such as a Wikipedia article.
fn entity(link: &Link) -> Option<Entity> {
    let url = AbsoluteUrl::parse(&link.href).ok()?;
    let host = url.host();
    let known = ENTITY_HOSTS
        .iter()
        .any(|known| host == *known || host.ends_with(&format!(".{known}")));
    let name = link.text.trim();
    (known && !name.is_empty()).then(|| Entity::new(name, EntityRole::Mention, vec![url]))
}

/// `src` as a full address: kept when it is one, on the site's host when
/// it starts at the root, and beside the page at `path` otherwise.
fn full_address(config: &Config, path: &str, src: &str) -> Result<AbsoluteUrl, UserMessage> {
    if let Ok(url) = AbsoluteUrl::parse(src) {
        return Ok(url);
    }
    let origin = config.address().as_str().trim_end_matches('/');
    let text = match src.strip_prefix("./").unwrap_or(src) {
        rooted if rooted.starts_with('/') => format!("{origin}{rooted}"),
        beside => format!("{origin}{path}{beside}"),
    };
    AbsoluteUrl::parse(&text).map_err(|_| build_message::address(&text))
}

#[cfg(test)]
#[path = "build_schema_tests.rs"]
mod tests;
