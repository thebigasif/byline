//! One problem the linter found, ready to show to a writer.

use khalid_core::{Locale, Slug};
use serde::{Serialize, Serializer};

use crate::{Message, RuleId, Severity};

/// Where a problem is: the post, and the thing in it when there is one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Location {
    /// The post's language.
    pub locale: Locale,
    /// The post's address.
    pub slug: Slug,
    /// The link, image or phrase the problem is about, if not the whole post.
    pub subject: Option<String>,
}

/// A rule's finding with its severity and its message.
///
/// Only the [`Linter`](crate::Linter) makes findings, and it never makes one
/// for a rule that is [`Severity::Off`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    rule: RuleId,
    severity: Severity,
    location: Location,
    message: Message,
    suggestions: Vec<String>,
}

impl Finding {
    pub(crate) fn new(
        rule: RuleId,
        severity: Severity,
        location: Location,
        message: Message,
        suggestions: Vec<String>,
    ) -> Self {
        Self {
            rule,
            severity,
            location,
            message,
            suggestions,
        }
    }

    /// Choices for the fix, best first, such as the addresses of related
    /// posts to link to. Empty for most rules.
    pub fn suggestions(&self) -> &[String] {
        &self.suggestions
    }

    /// The rule that found the problem.
    pub fn rule(&self) -> RuleId {
        self.rule
    }

    /// Whether the problem blocks publishing or is a warning.
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// Where the problem is.
    pub fn location(&self) -> &Location {
        &self.location
    }

    /// What is wrong, why it matters and what to do.
    pub fn message(&self) -> &Message {
        &self.message
    }
}

/// The JSON shape of a finding: one flat object with the rule, where it is,
/// the three sentences and the guide, so the CLI, the API and the admin
/// read the same fields.
#[derive(Serialize)]
struct FindingBody<'a> {
    rule: RuleId,
    severity: Severity,
    locale: &'a str,
    slug: &'a str,
    subject: Option<&'a str>,
    what: &'a str,
    why: &'a str,
    fix: &'a str,
    guide: &'a str,
    suggestions: &'a [String],
}

impl Serialize for Finding {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        FindingBody {
            rule: self.rule,
            severity: self.severity,
            locale: self.location.locale.as_str(),
            slug: self.location.slug.as_str(),
            subject: self.location.subject.as_deref(),
            what: self.message.what(),
            why: self.message.why(),
            fix: self.message.fix(),
            guide: self.message.guide().as_str(),
            suggestions: &self.suggestions,
        }
        .serialize(serializer)
    }
}
