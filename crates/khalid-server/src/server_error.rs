//! Why the server could not hold its pages or keep serving.

use crate::PagePath;

/// Why the server could not hold its pages or keep serving.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServerError {
    /// Two pages claim one address.
    #[error("two pages claim the address {path}")]
    DuplicatePage {
        /// The address both claim.
        path: PagePath,
    },
}
