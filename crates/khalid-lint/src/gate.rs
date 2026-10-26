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

/// Whatever decides if a post may go live, as every publish path holds it.
///
/// `khalid build` and the Admin API's publish path take a `&dyn PublishGate`
/// rather than a [`Linter`], so both ask the same question and a test can
/// stand in for the rules. [`Linter`] is the gate in the product.
pub trait PublishGate {
    /// The decision for `post`, read against the rest of `site`.
    fn verdict(&self, post: &LintPost, site: &dyn SiteView) -> Verdict;

    /// Whether [`PublishGate::verdict`] would refuse `post`. A gate may
    /// answer faster than a full verdict, which carries every warning too.
    fn refuses(&self, post: &LintPost, site: &dyn SiteView) -> bool {
        !self.verdict(post, site).allows_publishing()
    }
}

impl PublishGate for Linter {
    fn verdict(&self, post: &LintPost, site: &dyn SiteView) -> Verdict {
        self.gate(post, site)
    }

    fn refuses(&self, post: &LintPost, site: &dyn SiteView) -> bool {
        Self::refuses(self, post, site)
    }
}
