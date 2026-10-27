//! One check `khalid doctor` runs on a site folder, and what it found.

use khalid_core::UserMessage;
use serde::Serialize;

/// The checks, in the order doctor runs and prints them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Check {
    /// `site.toml` loads into typed settings.
    Settings,
    /// `routes.toml`, or the defaults, gives every post an address.
    Routes,
    /// Every content file and every block in it reads.
    Content,
    /// `data/site.db`, when there is one, opens.
    Database,
    /// `theme/tokens.toml`, when there is one, reads and is readable.
    Theme,
    /// Khalid may write to the folders it saves into.
    Permissions,
}

impl Check {
    /// The check's name at the start of a line, such as `Settings`.
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Settings => "Settings",
            Self::Routes => "Routes",
            Self::Content => "Content",
            Self::Database => "Database",
            Self::Theme => "Theme",
            Self::Permissions => "Permissions",
        }
    }
}

/// How a check ended, as `--json` spells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Status {
    /// Nothing to do.
    Ok,
    /// The person must fix something.
    Problem,
    /// Not run, because a check it needs found a problem.
    Skipped,
}

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// All is well, said in one sentence.
    Ok(String),
    /// Something to fix, as what, why and fix.
    Problem(UserMessage),
    /// Not run, with the reason in one sentence.
    Skipped(String),
}

/// One check and what it found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    pub(crate) check: Check,
    pub(crate) outcome: Outcome,
}

impl Finding {
    /// A check that found nothing to do.
    pub(crate) fn ok(check: Check, summary: impl Into<String>) -> Self {
        Self {
            check,
            outcome: Outcome::Ok(summary.into()),
        }
    }

    /// A check that found something to fix.
    pub(crate) fn problem(check: Check, message: UserMessage) -> Self {
        Self {
            check,
            outcome: Outcome::Problem(message),
        }
    }

    /// A check that did not run.
    pub(crate) fn skipped(check: Check, reason: impl Into<String>) -> Self {
        Self {
            check,
            outcome: Outcome::Skipped(reason.into()),
        }
    }

    /// How the check ended.
    pub(crate) fn status(&self) -> Status {
        match self.outcome {
            Outcome::Ok(_) => Status::Ok,
            Outcome::Problem(_) => Status::Problem,
            Outcome::Skipped(_) => Status::Skipped,
        }
    }
}
