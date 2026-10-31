//! Rules about how a post shows in search results: its title and description.

use crate::{Hit, LintPost, Rule, RuleId, Severity, SiteView};

/// The field's text, trimmed, if it has any.
fn filled(field: Option<&str>) -> Option<&str> {
    field.map(str::trim).filter(|text| !text.is_empty())
}

/// One hit when `field` holds more than `max` characters.
fn too_long(field: Option<&str>, max: usize) -> Vec<Hit> {
    let long = filled(field).is_some_and(|text| text.chars().count() > max);
    long.then(Hit::post).into_iter().collect()
}

/// One hit per other post in the same language whose `field` matches this
/// post's, ignoring case, named by that post's address.
fn duplicates(
    post: &LintPost,
    site: &dyn SiteView,
    field: impl Fn(&LintPost) -> Option<&str>,
) -> Vec<Hit> {
    let Some(own) = filled(field(post)).map(str::to_lowercase) else {
        return Vec::new();
    };
    site.posts()
        .iter()
        .filter(|other| other.locale == post.locale && other.slug != post.slug)
        .filter(|other| filled(field(other)).is_some_and(|text| text.to_lowercase() == own))
        .map(|other| Hit::about(other.slug.as_str()))
        .collect()
}

/// A post with no description.
#[derive(Debug)]
pub(crate) struct NoDescription;

impl Rule for NoDescription {
    fn id(&self) -> RuleId {
        RuleId::new("no-description")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        let missing = filled(post.description.as_deref()).is_none();
        missing.then(Hit::post).into_iter().collect()
    }
}

/// A title longer than search results show.
#[derive(Debug)]
pub(crate) struct LongTitle {
    /// The most characters, from `Thresholds::title_max_chars`.
    pub(crate) max: usize,
}

impl Rule for LongTitle {
    fn id(&self) -> RuleId {
        RuleId::new("long-title")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        too_long(post.title.as_deref(), self.max)
    }
}

/// A description longer than search results show.
#[derive(Debug)]
pub(crate) struct LongDescription {
    /// The most characters, from `Thresholds::description_max_chars`.
    pub(crate) max: usize,
}

impl Rule for LongDescription {
    fn id(&self) -> RuleId {
        RuleId::new("long-description")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        too_long(post.description.as_deref(), self.max)
    }
}

/// One hit when `field` has text and holds fewer than `min` characters. An
/// empty field is left to the rule about a missing one.
fn too_short(field: Option<&str>, min: usize) -> Vec<Hit> {
    let short = filled(field).is_some_and(|text| text.chars().count() < min);
    short.then(Hit::post).into_iter().collect()
}

/// A title too short to say what the post is about.
#[derive(Debug)]
pub(crate) struct ShortTitle {
    /// The fewest characters, from `Thresholds::title_min_chars`.
    pub(crate) min: usize,
}

impl Rule for ShortTitle {
    fn id(&self) -> RuleId {
        RuleId::new("short-title")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        too_short(post.title.as_deref(), self.min)
    }
}

/// A description too short to sum up the post in search results.
#[derive(Debug)]
pub(crate) struct ShortDescription {
    /// The fewest characters, from `Thresholds::description_min_chars`.
    pub(crate) min: usize,
}

impl Rule for ShortDescription {
    fn id(&self) -> RuleId {
        RuleId::new("short-description")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, _site: &dyn SiteView) -> Vec<Hit> {
        too_short(post.description.as_deref(), self.min)
    }
}

/// Another post in the same language with the same title.
#[derive(Debug)]
pub(crate) struct DuplicateTitle;

impl Rule for DuplicateTitle {
    fn id(&self) -> RuleId {
        RuleId::new("duplicate-title")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        duplicates(post, site, |p| p.title.as_deref())
    }
}

/// Another post in the same language with the same description.
#[derive(Debug)]
pub(crate) struct DuplicateDescription;

impl Rule for DuplicateDescription {
    fn id(&self) -> RuleId {
        RuleId::new("duplicate-description")
    }

    fn default_severity(&self) -> Severity {
        Severity::Warn
    }

    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit> {
        duplicates(post, site, |p| p.description.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_minimum_itself_is_long_enough_and_counts_characters() {
        assert_eq!(too_short(Some("Über Links"), 10), []);
        assert_eq!(too_short(Some("Über Link"), 10), [Hit::post()]);
        assert_eq!(too_short(Some("  Über Links  "), 11), [Hit::post()]);
    }

    #[test]
    fn an_empty_field_is_left_to_the_rule_about_a_missing_one() {
        assert_eq!(too_short(None, 20), []);
        assert_eq!(too_short(Some("   "), 20), []);
    }
}
