//! The fact-check block: `ClaimReview`.

use khalid_core::AbsoluteUrl;

use crate::schema::{node_id, publisher_id};
use crate::{Rating, SchemaNode, SchemaValue, SeoError, SeoPage, SeoSite};

/// A fact-check block: one claim the post checks, and its verdict.
///
/// Google stopped showing fact-check results in 2025, so no rule checks
/// it; the claim, the score and the verdict in words are required, as
/// fact-check tools that read schema.org still need all three.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ClaimReview {
    /// The claim, as it was made.
    pub claim: String,
    /// The verdict as a score, such as 1 out of 5.
    pub rating: Rating,
    /// The verdict in words, such as `False` or `Mostly true`.
    pub verdict: String,
}

impl ClaimReview {
    /// A check of `claim`, scored `rating` and summed up as `verdict`.
    pub fn new(claim: &str, rating: Rating, verdict: &str) -> Self {
        Self {
            claim: claim.to_owned(),
            rating,
            verdict: verdict.to_owned(),
        }
    }
}

/// The `ClaimReview` at `<page>#claim-<number>`: its `url` is the post,
/// its author the site's publisher, and its date the post's.
pub(crate) fn claim_review_node(
    review: &ClaimReview,
    (site, page): (&SeoSite, &SeoPage),
    url: &AbsoluteUrl,
    number: usize,
) -> Result<SchemaNode, SeoError> {
    let rating = review
        .rating
        .node("Rating")
        .with("alternateName", SchemaValue::text(&review.verdict));
    let mut node = SchemaNode::new("ClaimReview")
        .with_id(node_id(url, &format!("claim-{number}"))?)
        .with("url", SchemaValue::Url(url.clone()))
        .with("claimReviewed", SchemaValue::text(&review.claim))
        .with("reviewRating", SchemaValue::Node(Box::new(rating)))
        .with("author", SchemaValue::Reference(publisher_id(site)?));
    if let Some(published) = page.published {
        node.set("datePublished", SchemaValue::DateTime(published));
    }
    Ok(node)
}
