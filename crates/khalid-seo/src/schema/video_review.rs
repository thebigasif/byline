//! The video and review blocks: `VideoObject` and `Review`.

use khalid_core::AbsoluteUrl;

use crate::schema::node_id;
use crate::{SchemaNode, SchemaValue, SeoError, Video};

/// A score out of a best possible score, such as 4.5 out of 5.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rating {
    value: f64,
    best: f64,
}

impl Rating {
    /// `value` out of `best`; the worst possible score is 0.
    ///
    /// # Errors
    ///
    /// [`SeoError::Rating`] when `best` is not above 0, or `value` is below
    /// 0, above `best`, or not a number.
    pub fn new(value: f64, best: f64) -> Result<Self, SeoError> {
        if best > 0.0 && (0.0..=best).contains(&value) {
            Ok(Self { value, best })
        } else {
            Err(SeoError::Rating { value, best })
        }
    }

    /// The rating as a node of `kind`, `Rating` or `AggregateRating`.
    pub(crate) fn node(self, kind: &str) -> SchemaNode {
        SchemaNode::new(kind)
            .with("ratingValue", SchemaValue::Number(self.value))
            .with("bestRating", SchemaValue::Number(self.best))
            .with("worstRating", SchemaValue::Integer(0))
    }
}

/// The average of many readers' ratings.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct AggregateRating {
    /// The average.
    pub rating: Rating,
    /// How many ratings it averages.
    pub count: u32,
}

impl AggregateRating {
    /// An average of `count` ratings.
    pub fn new(rating: Rating, count: u32) -> Self {
        Self { rating, count }
    }

    /// The average as an `AggregateRating` node with its `ratingCount`.
    pub(crate) fn node(self) -> SchemaNode {
        let count = SchemaValue::Integer(i64::from(self.count));
        self.rating
            .node("AggregateRating")
            .with("ratingCount", count)
    }
}

/// A review block: one person's verdict on one thing.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Review {
    /// The schema.org type of the thing reviewed, such as `Product` or `Book`.
    pub item_type: String,
    /// The thing's name.
    pub item_name: String,
    /// The reviewer's score.
    pub rating: Rating,
    /// The reviewer's name.
    pub author: String,
    /// The review's text.
    pub body: String,
    /// Readers' average rating of the thing, if the site collects them.
    pub aggregate: Option<AggregateRating>,
}

impl Review {
    /// A review of `item_name`, a schema.org `item_type`, by `author`.
    pub fn new(item_type: &str, item_name: &str, rating: Rating, author: &str) -> Self {
        Self {
            item_type: item_type.to_owned(),
            item_name: item_name.to_owned(),
            rating,
            author: author.to_owned(),
            body: String::new(),
            aggregate: None,
        }
    }
}

/// The `VideoObject` at `<page>#video-<number>`, from the video's own data.
pub(crate) fn video_node(
    video: &Video,
    page: &AbsoluteUrl,
    number: usize,
) -> Result<SchemaNode, SeoError> {
    let mut node = SchemaNode::new("VideoObject")
        .with_id(node_id(page, &format!("video-{number}"))?)
        .with("name", SchemaValue::text(&video.title))
        .with("description", SchemaValue::text(&video.description))
        .with("thumbnailUrl", SchemaValue::Url(video.thumbnail.clone()));
    if let Some(uploaded) = video.uploaded {
        node.set("uploadDate", SchemaValue::DateTime(uploaded));
    }
    if let Some(duration) = video.duration {
        node.set("duration", SchemaValue::Duration(duration));
    }
    if let Some(file) = video.file() {
        node.set("contentUrl", SchemaValue::Url(file.clone()));
    }
    if let Some(player) = video.player() {
        node.set("embedUrl", SchemaValue::Url(player.clone()));
    }
    Ok(node)
}

/// The `Review` at `<page>#review-<number>`, with the readers' average
/// inside the item reviewed, where Google reads it.
pub(crate) fn review_node(
    review: &Review,
    page: &AbsoluteUrl,
    number: usize,
) -> Result<SchemaNode, SeoError> {
    let mut item =
        SchemaNode::new(&review.item_type).with("name", SchemaValue::text(&review.item_name));
    if let Some(aggregate) = review.aggregate {
        item.set(
            "aggregateRating",
            SchemaValue::Node(Box::new(aggregate.node())),
        );
    }
    let author = SchemaNode::new("Person").with("name", SchemaValue::text(&review.author));
    let mut node = SchemaNode::new("Review")
        .with_id(node_id(page, &format!("review-{number}"))?)
        .with("itemReviewed", SchemaValue::Node(Box::new(item)))
        .with(
            "reviewRating",
            SchemaValue::Node(Box::new(review.rating.node("Rating"))),
        )
        .with("author", SchemaValue::Node(Box::new(author)));
    if !review.body.is_empty() {
        node.set("reviewBody", SchemaValue::text(&review.body));
    }
    Ok(node)
}
