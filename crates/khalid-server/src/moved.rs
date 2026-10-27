//! An old address that sends readers on to a new one.

use crate::PagePath;

/// An address that moved for good, such as a post's alias: a request for
/// `from` gets 301 with `to` in `Location`.
///
/// Both are paths on this site, so a move never sends a reader elsewhere.
/// `to` need not be a page this server holds: on a headless site another
/// app renders it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moved {
    from: PagePath,
    to: PagePath,
}

impl Moved {
    /// `from` moved to `to`.
    pub fn new(from: PagePath, to: PagePath) -> Self {
        Self { from, to }
    }

    /// The old address.
    pub fn from(&self) -> &PagePath {
        &self.from
    }

    /// The address it moved to.
    pub fn to(&self) -> &PagePath {
        &self.to
    }
}
