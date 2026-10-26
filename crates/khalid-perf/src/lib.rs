//! Generated large sites and the measures behind the performance budgets.
//!
//! The budgets in `docs/standards/performance.md` are measured on a site of
//! 1,000 posts and one of 10,000. Committing that many files would bury the
//! repository, so this crate generates them: [`Fixture`] gives every post's
//! file name and text, the same bytes on every machine.

mod fixture;
mod words;

pub use fixture::Fixture;
