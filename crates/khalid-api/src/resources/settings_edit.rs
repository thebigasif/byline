//! An edit to a post's settings: only the settings it names change.

use chrono::NaiveDate;
use khalid_core::{AbsoluteUrl, RobotsDirectives, Slug};
use serde::{Deserialize, Deserializer};
use utoipa::ToSchema;

use super::{Alias, FeatureImage, NonBlank, PostSettings, ReviewState, ShareCards};

/// New values for some of a post's settings. A setting left out keeps
/// its value, and `null` clears one that may be empty.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SettingsEdit {
    /// A new date, as `YYYY-MM-DD`, or `null` for none.
    #[serde(default)]
    #[schema(value_type = Option<String>, format = Date, example = "2026-11-02")]
    date: Change<NaiveDate>,
    /// The other authors' slugs, all of them.
    #[schema(value_type = Option<Vec<String>>, example = json!(["mira"]))]
    co_authors: Option<Vec<Slug>>,
    /// A new excerpt, or `null` for none.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    excerpt: Change<NonBlank>,
    /// A new feature image, or `null` for none.
    #[serde(default)]
    #[schema(value_type = Option<FeatureImage>)]
    feature_image: Change<FeatureImage>,
    /// Whether the post is featured.
    featured: Option<bool>,
    /// `draft`, `in_review` or `ready`.
    review_state: Option<ReviewState>,
    /// A new canonical address, or `null` for the post's own.
    #[serde(default)]
    #[schema(value_type = Option<String>, example = "https://example.com/original/")]
    canonical: Change<AbsoluteUrl>,
    /// New robots directives, or `null` for the site's.
    #[serde(default)]
    #[schema(value_type = Option<String>, example = "noindex, nofollow")]
    robots: Change<RobotsDirectives>,
    /// Both share cards; a card left out uses the post's own title,
    /// excerpt and image.
    share_cards: Option<ShareCards>,
    /// Every old address that redirects here.
    #[schema(value_type = Option<Vec<String>>, example = json!(["/2019/05/old-title/"]))]
    aliases: Option<Vec<Alias>>,
}

impl SettingsEdit {
    /// `settings` with the ones this edit names put in their place.
    #[must_use]
    pub fn applied_to(self, settings: PostSettings) -> PostSettings {
        PostSettings {
            date: self.date.applied_to(settings.date),
            co_authors: self.co_authors.unwrap_or(settings.co_authors),
            excerpt: self.excerpt.applied_to(settings.excerpt),
            feature_image: self.feature_image.applied_to(settings.feature_image),
            featured: self.featured.unwrap_or(settings.featured),
            review_state: self.review_state.unwrap_or(settings.review_state),
            canonical: self.canonical.applied_to(settings.canonical),
            robots: self.robots.applied_to(settings.robots),
            share_cards: self.share_cards.unwrap_or(settings.share_cards),
            aliases: self.aliases.unwrap_or(settings.aliases),
            ..settings
        }
    }
}

/// What an edit does to a setting that may be empty.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) enum Change<T> {
    /// Left out: the setting keeps its value.
    #[default]
    Keep,
    /// Sent as `null`: the setting is emptied.
    Clear,
    /// Sent with a value.
    Set(T),
}

impl<T> Change<T> {
    /// The setting's value after the change, from its value `before`.
    pub(super) fn applied_to(self, before: Option<T>) -> Option<T> {
        match self {
            Self::Keep => before,
            Self::Clear => None,
            Self::Set(value) => Some(value),
        }
    }
}

/// Serde calls this only for a field that is present, so it reads `null`
/// or a value; a field left out is [`Change::Keep`] through `default`.
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Change<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Option::<T>::deserialize(deserializer).map(|value| value.map_or(Self::Clear, Self::Set))
    }
}
