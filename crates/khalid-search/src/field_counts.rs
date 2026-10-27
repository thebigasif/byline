//! A number for each part of a page that search reads.

/// A count for each part of a page: how many words it has, or how often one
/// term appears in it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct FieldCounts {
    /// In the title.
    pub title: u32,
    /// In the headings.
    pub headings: u32,
    /// In the body.
    pub body: u32,
}

impl FieldCounts {
    /// The counts for the title, the headings and the body.
    pub fn new(title: u32, headings: u32, body: u32) -> Self {
        Self {
            title,
            headings,
            body,
        }
    }

    /// Each count added to the same count of `other`, stopping at the
    /// largest number rather than wrapping.
    #[must_use]
    pub fn plus(self, other: Self) -> Self {
        Self {
            title: self.title.saturating_add(other.title),
            headings: self.headings.saturating_add(other.headings),
            body: self.body.saturating_add(other.body),
        }
    }
}
