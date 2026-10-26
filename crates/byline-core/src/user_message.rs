//! The message a person reads when something needs their attention.

use std::borrow::Cow;
use std::fmt;

use serde::Serialize;

/// What happened, why it matters and what to do, as three plain sentences.
///
/// Every message a person can see takes this shape (RULES.md BL-ERR-4). Library
/// crates return typed errors, and the binary and the API turn them into a
/// `UserMessage` at the edge. The same struct serialises into the command
/// line's `--json` output and the API's problem details, with the fields
/// `what`, `why` and `fix`. `Display` prints the three sentences on three lines.
///
/// # Examples
///
/// ```
/// use byline_core::UserMessage;
///
/// let message = UserMessage::new(
///     "This post has no date.",
///     "Google and readers use dates to judge freshness.",
///     "Add `date:` to the header of `posts/why-links-matter.md`.",
/// );
/// assert_eq!(message.why(), "Google and readers use dates to judge freshness.");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserMessage {
    what: Cow<'static, str>,
    why: Cow<'static, str>,
    fix: Cow<'static, str>,
}

impl UserMessage {
    /// Builds a message from its three sentences, each written for a person.
    pub fn new(
        what: impl Into<Cow<'static, str>>,
        why: impl Into<Cow<'static, str>>,
        fix: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            what: what.into(),
            why: why.into(),
            fix: fix.into(),
        }
    }

    /// What happened.
    pub fn what(&self) -> &str {
        &self.what
    }

    /// Why it matters to the person reading.
    pub fn why(&self) -> &str {
        &self.why
    }

    /// What to do next.
    pub fn fix(&self) -> &str {
        &self.fix
    }
}

impl fmt::Display for UserMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\n{}\n{}", self.what, self.why, self.fix)
    }
}
