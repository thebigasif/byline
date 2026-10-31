//! The product block: `Product`.

use khalid_core::AbsoluteUrl;

use crate::schema::node_id;
use crate::{AggregateRating, Offer, Rating, SchemaNode, SchemaValue, SeoError};

/// One person's score of a product, written inside it as a `Review`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ProductReview {
    /// The score.
    pub rating: Rating,
    /// Who gave it, usually the post's author.
    pub author: String,
}

impl ProductReview {
    /// `author`'s score of `rating`.
    pub fn new(rating: Rating, author: &str) -> Self {
        Self {
            rating,
            author: author.to_owned(),
        }
    }
}

/// A product block: a thing the post recommends or sells.
///
/// Google shows a product only with a price, a review or readers' average,
/// so a product with none of them blocks publishing.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Product {
    /// The product.
    pub name: String,
    /// What it is, in plain text.
    pub description: String,
    /// Pictures of it.
    pub images: Vec<AbsoluteUrl>,
    /// Its price, if the post gives one.
    pub offer: Option<Offer>,
    /// The writer's score, if the post gives one.
    pub review: Option<ProductReview>,
    /// Readers' average score, if the site collects them.
    pub aggregate: Option<AggregateRating>,
}

impl Product {
    /// A product called `name`, with nothing else known yet.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            description: String::new(),
            images: Vec::new(),
            offer: None,
            review: None,
            aggregate: None,
        }
    }
}

/// The `Product` at `<page>#product-<number>`.
pub(crate) fn product_node(
    product: &Product,
    page: &AbsoluteUrl,
    number: usize,
) -> Result<SchemaNode, SeoError> {
    let mut node = SchemaNode::new("Product")
        .with_id(node_id(page, &format!("product-{number}"))?)
        .with("name", SchemaValue::text(&product.name));
    if !product.description.is_empty() {
        node.set("description", SchemaValue::text(&product.description));
    }
    if !product.images.is_empty() {
        let images = product.images.iter().cloned().map(SchemaValue::Url);
        node.set("image", SchemaValue::List(images.collect()));
    }
    if let Some(offer) = &product.offer {
        node.set("offers", SchemaValue::Node(Box::new(offer.node())));
    }
    if let Some(review) = &product.review {
        let author = SchemaNode::new("Person").with("name", SchemaValue::text(&review.author));
        let review = SchemaNode::new("Review")
            .with(
                "reviewRating",
                SchemaValue::Node(Box::new(review.rating.node("Rating"))),
            )
            .with("author", SchemaValue::Node(Box::new(author)));
        node.set("review", SchemaValue::Node(Box::new(review)));
    }
    if let Some(aggregate) = product.aggregate {
        node.set(
            "aggregateRating",
            SchemaValue::Node(Box::new(aggregate.node())),
        );
    }
    Ok(node)
}
