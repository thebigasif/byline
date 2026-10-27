//! The features that need a running server, switched off in static mode.

use serde::Serialize;

/// One feature a static site does not have (plan section 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ServerFeature {
    /// Readers leaving comments under a post.
    Comments,
    /// Readers signing up by email, and gated content behind a sign-up.
    Newsletter,
    /// Contact forms and other forms that send to the site.
    Forms,
    /// Search as you type, and the search page's results.
    Search,
    /// The admin in the browser, on the published site.
    Admin,
    /// A post going live by itself at a set time.
    ScheduledPublish,
}

impl ServerFeature {
    /// Every feature static mode switches off, in the order Khalid lists them.
    pub const ALL: [Self; 6] = [
        Self::Comments,
        Self::Newsletter,
        Self::Forms,
        Self::Search,
        Self::Admin,
        Self::ScheduledPublish,
    ];

    /// The feature's name in `--json` output, such as `scheduled_publish`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Comments => "comments",
            Self::Newsletter => "newsletter",
            Self::Forms => "forms",
            Self::Search => "search",
            Self::Admin => "admin",
            Self::ScheduledPublish => "scheduled_publish",
        }
    }
}
