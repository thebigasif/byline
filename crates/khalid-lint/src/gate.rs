//! The publish gate: whether a post may go live, and why not.

use serde::Serialize;

use crate::{LintPost, Linter, Report, SiteView};

/// What the publish gate decided for one post, with every finding behind it.
///
/// Serialised as `{"decision": "publish" | "refuse", "report": {...}}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
#[must_use]
pub enum Verdict {
    /// Nothing blocks the post. The report holds its warnings, if any.
    Publish {
        /// The post's findings, all of them warnings.
        report: Report,
    },
    /// At least one finding blocks the post. Publishing must stop.
    Refuse {
        /// The post's findings, blocks and warnings together.
        report: Report,
    },
}

impl Verdict {
    /// Whether the post may be published.
    pub fn allows_publishing(&self) -> bool {
        matches!(self, Self::Publish { .. })
    }

    /// The findings behind the decision.
    pub fn report(&self) -> &Report {
        match self {
            Self::Publish { report } | Self::Refuse { report } => report,
        }
    }
}

impl Linter {
    /// The publish gate the server, the CLI and the admin all call: runs the
    /// post rules on `post` and refuses it when any finding blocks.
    pub fn gate(&self, post: &LintPost, site: &dyn SiteView) -> Verdict {
        let report = self.lint_post(post, site);
        if report.blocks() {
            Verdict::Refuse { report }
        } else {
            Verdict::Publish { report }
        }
    }
}
