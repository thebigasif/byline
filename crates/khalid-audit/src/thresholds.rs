//! The limits the audit's checks measure against.

/// The limits a site may change for the audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Thresholds {
    pub(crate) most_links: usize,
}

impl Default for Thresholds {
    /// 3,000 links on a page, as Semrush counts.
    fn default() -> Self {
        Self { most_links: 3_000 }
    }
}

impl Thresholds {
    /// The same limits with at most `links` links on a page.
    #[must_use]
    pub fn with_most_links(self, links: usize) -> Self {
        Self { most_links: links }
    }
}
