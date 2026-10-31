//! One issue the audit found, on one address.

use khalid_core::AbsoluteUrl;

use crate::Check;

/// A check that failed on one address, with what it saw there.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Issue {
    check: Check,
    url: AbsoluteUrl,
    detail: String,
}

impl Issue {
    /// `check` failed on `url`, and `detail` says what it saw, such as the
    /// link it means. `detail` may be empty.
    pub fn new(check: Check, url: &AbsoluteUrl, detail: impl Into<String>) -> Self {
        Self {
            check,
            url: url.clone(),
            detail: detail.into(),
        }
    }

    /// The check that failed.
    pub fn check(&self) -> Check {
        self.check
    }

    /// The address it failed on.
    pub fn url(&self) -> &AbsoluteUrl {
        &self.url
    }

    /// What the check saw there, such as the link or the length, or
    /// nothing.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}
