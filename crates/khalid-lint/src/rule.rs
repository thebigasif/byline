//! What a lint rule is.

use std::fmt;

use serde::Serialize;

use crate::{LintPost, Severity, SiteView};

/// The stable name of a rule, such as `no-author`.
///
/// It is the key in `lint.toml` and in the message catalogue, and it never
/// changes once released, because sites refer to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct RuleId(&'static str);

impl RuleId {
    /// Names a rule.
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    /// The rule's name as text.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// One problem a rule found in a post.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Hit {
    subject: Option<String>,
}

impl Hit {
    /// A problem with the post as a whole.
    pub fn post() -> Self {
        Self::default()
    }

    /// A problem with one thing in the post, such as a link or an image,
    /// named so the writer can find it.
    pub fn about(subject: impl Into<String>) -> Self {
        Self {
            subject: Some(subject.into()),
        }
    }

    /// The thing the problem is about, if it is not the whole post.
    pub fn subject(&self) -> Option<&str> {
        self.subject.as_deref()
    }
}

/// A check the linter runs on every post.
///
/// A rule only finds problems. Its message comes from the catalogue under its
/// [`RuleId`], and its severity from `lint.toml` or its default, so a rule
/// never decides on its own whether a post is blocked.
pub trait Rule: fmt::Debug + Send + Sync {
    /// The rule's stable name.
    fn id(&self) -> RuleId;

    /// The severity when `lint.toml` does not set one.
    fn default_severity(&self) -> Severity;

    /// The problems this rule finds in `post`, reading the rest of the site
    /// through `site`. An empty list means the post passes.
    fn check(&self, post: &LintPost, site: &dyn SiteView) -> Vec<Hit>;
}
