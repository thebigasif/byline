//! Where the Admin API finds Search Console follow-ups (GEO M3): the long,
//! conversational searches that showed each post, the way people ask an AI
//! assistant, and the heading of the post that answers each.

use std::error::Error;

use chrono::NaiveDate;
use khalid_core::UserMessage;
use serde::Serialize;
use utoipa::ToSchema;

use crate::content_store::BoxFuture;

/// Finds the follow-ups of the site's posts. The `khalid` binary
/// implements it with the same finder `khalid search-console follow-ups`
/// runs; [`NoSearchConsole`] stands in for a site with no connection.
pub trait FollowUpFinder: Send + Sync {
    /// The long searches of the last 28 days whose numbers are final, per
    /// post.
    ///
    /// # Errors
    ///
    /// [`FollowUpError::NotConnected`] when the site has no Search Console
    /// connection, [`FollowUpError::Google`] when Google refuses or cannot
    /// be reached, and [`FollowUpError::Internal`] when the site's posts
    /// cannot be read.
    fn follow_ups(&self) -> BoxFuture<'_, Result<FollowUps, FollowUpError>>;
}

/// The long searches that showed the site's posts in a month.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct FollowUps {
    /// The Search Console property the numbers come from.
    #[schema(example = "sc-domain:example.com")]
    pub property: String,
    /// The first day looked at.
    pub first: NaiveDate,
    /// The last day looked at: the last day whose numbers are final.
    pub last: NaiveDate,
    /// The fewest words a search had to have to count.
    #[schema(example = 8)]
    pub words: usize,
    /// The posts the searches showed, the one with the most shown search
    /// first.
    pub posts: Vec<FollowUpPost>,
}

/// One post and the long searches that showed it.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct FollowUpPost {
    /// The post's title.
    pub title: String,
    /// The post's full address.
    #[schema(example = "https://example.com/blog/what-a-sitemap-does/")]
    pub url: String,
    /// Its long searches, the most shown first.
    pub searches: Vec<FollowUp>,
}

/// One long search and how it did.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct FollowUp {
    /// The search as people typed it.
    #[schema(example = "do i need a sitemap if my blog has only ten posts")]
    pub query: String,
    /// Clicks from it to the post.
    pub clicks: f64,
    /// Times it showed the post.
    pub impressions: f64,
    /// The post's average position in its results, 1 at the top.
    pub position: f64,
    /// The heading of the post that answers it, or none yet.
    pub answered_by: Option<String>,
}

/// Why the follow-ups could not be found. The first two carry what
/// happened, why and what to do, for the person who asked.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FollowUpError {
    /// The site has no Search Console connection or no Google sign-in.
    #[error("Search Console is not set up: {}", .0.what())]
    NotConnected(UserMessage),
    /// Google refused, or could not be reached.
    #[error("Google gave no usable answer: {}", .0.what())]
    Google(UserMessage),
    /// The site's posts could not be read.
    #[error("the site's posts could not be read")]
    Internal(#[source] Box<dyn Error + Send + Sync>),
}

/// The finder of a site with no Search Console connection: every answer
/// says so.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoSearchConsole;

impl FollowUpFinder for NoSearchConsole {
    fn follow_ups(&self) -> BoxFuture<'_, Result<FollowUps, FollowUpError>> {
        let message = UserMessage::new(
            "This site is not connected to Google Search Console, Google's report of how a site does in search.",
            "Only the running khalid serve, with [search_console] in site.toml, can ask Google.",
            "Follow the steps in docs/deploy.md under Google Search Console, then send the request to khalid serve.",
        );
        Box::pin(async move { Err(FollowUpError::NotConnected(message)) })
    }
}
