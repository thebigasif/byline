//! A post's settings, from plan section 4.6: what the admin's settings
//! panel shows beside the editor.

use std::fmt;

use chrono::NaiveDate;
use khalid_core::{
    AbsoluteUrl, HeadCode, PasswordHash, PostAccess, RobotsDirectives, Slug, TemplateName,
};
use serde::{Deserialize, Serialize, Serializer};
use utoipa::ToSchema;
use utoipa::openapi::schema::{ObjectBuilder, Schema, Type};

use super::display_option;

/// Where a post stands in review, before it is published (KH-EDT-4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReviewState {
    /// Still being written.
    #[default]
    Draft,
    /// Waiting for an editor.
    InReview,
    /// An editor has said it may go live.
    Ready,
}

impl ReviewState {
    /// The name the API uses, such as `in_review`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::InReview => "in_review",
            Self::Ready => "ready",
        }
    }
}

/// Text with something in it other than spaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "String")]
pub struct NonBlank(String);

impl NonBlank {
    /// `text`, or `None` when it is empty or only spaces.
    pub fn try_new(text: &str) -> Option<Self> {
        (!text.trim().is_empty()).then(|| Self(text.to_owned()))
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for NonBlank {
    type Error = &'static str;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::try_new(&text).ok_or("this text must not be blank")
    }
}

/// The image at the top of a post and on its cards. The alt text is
/// required, as for every image (plan section 4.6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct FeatureImage {
    /// A media path such as `/media/anchor-text.jpg`, or a full address.
    pub src: NonBlank,
    /// What the image shows, for readers who cannot see it.
    pub alt: NonBlank,
    /// The line shown under the image, if any.
    #[serde(default)]
    pub caption: Option<NonBlank>,
}

impl FeatureImage {
    /// The image at `src`, described by `alt`, with no caption.
    pub fn new(src: NonBlank, alt: NonBlank) -> Self {
        let caption = None;
        Self { src, alt, caption }
    }
}

/// How a post looks when shared on one network. Anything left out comes
/// from the post's title, excerpt and feature image.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ShareCard {
    /// The headline on the card.
    #[serde(default)]
    pub title: Option<NonBlank>,
    /// The line under the headline.
    #[serde(default)]
    pub description: Option<NonBlank>,
    /// A media path or address, in place of the feature image.
    #[serde(default)]
    pub image: Option<NonBlank>,
}

/// The share cards for X and Facebook (plan section 4.6).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ShareCards {
    /// The card on X.
    #[serde(default)]
    pub x: Option<ShareCard>,
    /// The card on Facebook, and on other sites that read Open Graph.
    #[serde(default)]
    pub facebook: Option<ShareCard>,
}

/// An old address on this site that redirects to the post, such as
/// `/2019/05/old-title/`, so a moved post keeps its links.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "String")]
#[schema(value_type = String, example = "/2019/05/old-title/")]
pub struct Alias(String);

impl Alias {
    /// `path` as an alias, or `None` unless it starts with `/` and has no
    /// spaces, as front matter's `aliases` asks.
    pub fn try_new(path: &str) -> Option<Self> {
        (path.starts_with('/') && !path.contains(char::is_whitespace))
            .then(|| Self(path.to_owned()))
    }

    /// The path.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Alias {
    type Error = String;

    fn try_from(path: String) -> Result<Self, Self::Error> {
        Self::try_new(&path).ok_or_else(|| {
            format!("{path:?} is not a path; an alias is an old address on this site starting with /, such as /2019/05/old-title/")
        })
    }
}

/// A post's settings beside its title and body. Every setting has a
/// value: empty lists, `false`, `draft` and `null` when never set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, ToSchema)]
#[non_exhaustive]
pub struct PostSettings {
    /// The day the post is dated, as shown to readers.
    #[schema(value_type = Option<String>, format = Date, example = "2026-11-02")]
    pub date: Option<NaiveDate>,
    /// The other authors' slugs, after the author.
    #[serde(serialize_with = "display_all")]
    #[schema(value_type = Vec<String>, example = json!(["mira"]))]
    pub co_authors: Vec<Slug>,
    /// The summary for cards, feeds and search results.
    pub excerpt: Option<NonBlank>,
    /// The image at the top of the post.
    pub feature_image: Option<FeatureImage>,
    /// Whether the post is featured in lists and the featured channel.
    pub featured: bool,
    /// Where the post stands in review.
    pub review_state: ReviewState,
    /// The day the post last came up for review and was looked at (F3).
    /// A review that changes nothing sets this alone; it never moves
    /// `date`, and a real change is what sets `updated` in the front
    /// matter.
    #[schema(value_type = Option<String>, format = Date, example = "2026-05-02")]
    pub reviewed: Option<NaiveDate>,
    /// The address search engines should credit instead of this post.
    #[serde(serialize_with = "display_option")]
    #[schema(value_type = Option<String>, example = "https://example.com/original/")]
    pub canonical: Option<AbsoluteUrl>,
    /// The post's own robots directives, in place of the site's.
    #[serde(serialize_with = "display_option")]
    #[schema(value_type = Option<String>, example = "noindex, nofollow")]
    pub robots: Option<RobotsDirectives>,
    /// How the post looks when shared on X and Facebook.
    pub share_cards: ShareCards,
    /// Old addresses that redirect here.
    pub aliases: Vec<Alias>,
    /// Who may read the body: `public` or `members`.
    #[schema(schema_with = access_schema)]
    pub access: PostAccess,
    /// Whether a reader must give a password to read the body. The
    /// password itself is never sent back.
    #[serde(rename = "password_protected", serialize_with = "is_set")]
    #[schema(value_type = bool)]
    pub password: Option<PasswordHash>,
    /// The theme template the post is shown with, such as `wide` for
    /// `theme/templates/post-wide.html`, or `null` for `post.html`.
    #[serde(serialize_with = "display_option")]
    #[schema(value_type = Option<String>, example = "wide")]
    pub template: Option<TemplateName>,
    /// Checked HTML added to the post's page head, such as an analytics
    /// tag. Only owners and admins may change it.
    #[serde(serialize_with = "display_option")]
    #[schema(value_type = Option<String>, example = "<meta name=\"verify\" content=\"x\">")]
    pub head_code: Option<HeadCode>,
}

/// The schema for [`PostAccess`]: one of its names.
pub(super) fn access_schema() -> Schema {
    let names = PostAccess::ALL.map(PostAccess::as_str);
    let schema = ObjectBuilder::new()
        .schema_type(Type::String)
        .enum_values(Some(names))
        .description(Some(
            "Who may read the body: `public`, or `members`, who are signed in.",
        ));
    schema.into()
}

/// Writes whether there is a value, and never the value.
#[expect(
    clippy::ref_option,
    reason = "serde's serialize_with passes the field by reference"
)]
fn is_set<T, S: Serializer>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_bool(value.is_some())
}

/// Writes each value through its `Display`.
fn display_all<T: fmt::Display, S: Serializer>(
    values: &[T],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(values.iter().map(ToString::to_string))
}
