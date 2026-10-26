//! Which site a row belongs to.

/// The row id of a site in the `sites` table. Only [`Database::site`] makes
/// one, so a `SiteId` always names a site that exists in that database.
///
/// [`Database::site`]: crate::Database::site
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SiteId(i64);

impl SiteId {
    /// Wraps an id read from the `sites` table.
    pub(crate) fn from_row(id: i64) -> Self {
        Self(id)
    }

    /// The id as SQLite stores it, for binding into a query.
    pub(crate) fn as_row(self) -> i64 {
        self.0
    }
}
