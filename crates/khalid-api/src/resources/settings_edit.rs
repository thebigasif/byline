//! An edit to a post's settings: only the settings it names change.

use chrono::NaiveDate;
use khalid_core::{
    AbsoluteUrl, HeadCode, PasswordHash, PostAccess, RobotsDirectives, Slug, TemplateName,
};
use serde::{Deserialize, Deserializer};
use utoipa::ToSchema;

use super::post_settings::access_schema;
use super::{Alias, FeatureImage, NonBlank, PostSettings, ReviewState, ShareCards};
use crate::post_password::{HashPasswordError, NewPassword};

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
    /// The day the post was last looked over (F3), or `null` for never.
    /// Setting it records a review and nothing else: it never moves the
    /// post's date or its updated date.
    #[serde(default)]
    #[schema(value_type = Option<String>, format = Date, example = "2026-05-02")]
    reviewed: Change<NaiveDate>,
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
    /// `public` or `members`.
    #[schema(schema_with = access_schema)]
    access: Option<PostAccess>,
    /// A new password of 8 to 128 characters that readers must give, or
    /// `null` to remove it. It is kept only as a hash.
    #[serde(default)]
    #[schema(value_type = Option<String>, format = Password)]
    password: Change<NewPassword>,
    /// The new password's hash, made by [`SettingsEdit::hash_password`].
    #[serde(skip)]
    password_hash: Change<PasswordHash>,
    /// A theme template such as `wide`, or `null` for `post.html`.
    #[serde(default)]
    #[schema(value_type = Option<String>, example = "wide")]
    template: Change<TemplateName>,
    /// New head code, or `null` for none. Owners and admins only.
    #[serde(default)]
    #[schema(value_type = Option<String>, example = "<meta name=\"verify\" content=\"x\">")]
    head_code: Change<HeadCode>,
}

impl SettingsEdit {
    /// Whether the edit sends head code at all, changed or not.
    pub fn names_head_code(&self) -> bool {
        self.head_code != Change::Keep
    }

    /// Whether the edit changes head code that is `before` now.
    pub fn changes_head_code(&self, before: Option<&HeadCode>) -> bool {
        match &self.head_code {
            Change::Keep => false,
            Change::Clear => before.is_some(),
            Change::Set(code) => before != Some(code),
        }
    }

    /// Hashes a new password, or notes its removal, for
    /// [`applied_to`](Self::applied_to). Slow on purpose: call it on a
    /// blocking thread (BL-RUN-7).
    ///
    /// # Errors
    ///
    /// Returns [`HashPasswordError`] when Argon2id cannot hash it.
    pub fn hash_password(&mut self) -> Result<(), HashPasswordError> {
        self.password_hash = match &self.password {
            Change::Keep => Change::Keep,
            Change::Clear => Change::Clear,
            Change::Set(password) => Change::Set(password.hash()?),
        };
        Ok(())
    }

    /// `settings` with the ones this edit names put in their place. A new
    /// password counts only once [`hash_password`](Self::hash_password)
    /// has run.
    #[must_use]
    pub fn applied_to(self, settings: PostSettings) -> PostSettings {
        PostSettings {
            access: self.access.unwrap_or(settings.access),
            password: self.password_hash.applied_to(settings.password),
            template: self.template.applied_to(settings.template),
            head_code: self.head_code.applied_to(settings.head_code),
            date: self.date.applied_to(settings.date),
            co_authors: self.co_authors.unwrap_or(settings.co_authors),
            excerpt: self.excerpt.applied_to(settings.excerpt),
            feature_image: self.feature_image.applied_to(settings.feature_image),
            featured: self.featured.unwrap_or(settings.featured),
            review_state: self.review_state.unwrap_or(settings.review_state),
            reviewed: self.reviewed.applied_to(settings.reviewed),
            canonical: self.canonical.applied_to(settings.canonical),
            robots: self.robots.applied_to(settings.robots),
            share_cards: self.share_cards.unwrap_or(settings.share_cards),
            aliases: self.aliases.unwrap_or(settings.aliases),
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
