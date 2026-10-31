//! [`SchemaBlock`]: the editor blocks that write their own schema (task 1.7).

use crate::schema::event::event_node;
use crate::schema::faq::faq_node;
use crate::schema::howto::how_to_node;
use crate::schema::product::product_node;
use crate::schema::recipe::recipe_node;
use crate::schema::software::app_node;
use crate::schema::video_review::{review_node, video_node};
use crate::{
    Event, FaqItem, HowTo, Product, Recipe, Review, SchemaNode, SeoError, SeoPage, SeoSite,
    SoftwareApplication, Video,
};

/// An editor block on a page that writes its own schema node.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SchemaBlock {
    /// An FAQ block: `FAQPage`.
    Faq(Vec<FaqItem>),
    /// A how-to block: `HowTo`.
    HowTo(HowTo),
    /// A video embed: `VideoObject`.
    Video(Video),
    /// A review block: `Review`, with the readers' `AggregateRating`.
    Review(Review),
    /// A recipe block: `Recipe`.
    Recipe(Recipe),
    /// A product card: `Product`, with its price and ratings.
    Product(Product),
    /// A software block: `SoftwareApplication`, or a mobile or web app.
    Software(SoftwareApplication),
    /// An event block: `Event`.
    Event(Event),
}

/// The schema nodes of the page's blocks, in the order of the blocks. A
/// block numbered `n` on the page gets an id ending in `-n` where it needs one.
pub(crate) fn block_nodes(site: &SeoSite, page: &SeoPage) -> Result<Vec<SchemaNode>, SeoError> {
    let url = site.url_for(&page.language, &page.path)?;
    let mut nodes = Vec::new();
    for (number, block) in (1..).zip(&page.blocks) {
        match block {
            SchemaBlock::Faq(items) => nodes.extend(faq_node(items, &url)?),
            SchemaBlock::HowTo(how_to) => nodes.push(how_to_node(how_to, &url, number)?),
            SchemaBlock::Video(video) => nodes.push(video_node(video, &url, number)?),
            SchemaBlock::Review(review) => nodes.push(review_node(review, &url, number)?),
            SchemaBlock::Recipe(recipe) => nodes.push(recipe_node(recipe, page, &url, number)?),
            SchemaBlock::Product(product) => nodes.push(product_node(product, &url, number)?),
            SchemaBlock::Software(app) => nodes.push(app_node(app, &url, number)?),
            SchemaBlock::Event(event) => nodes.push(event_node(event, site, &url, number)?),
        }
    }
    Ok(nodes)
}
