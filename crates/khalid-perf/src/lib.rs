//! Generated large sites and the measures behind the performance budgets.
//!
//! The budgets in `docs/standards/performance.md` are measured on a site of
//! 1,000 posts and one of 10,000. Committing that many files would bury the
//! repository, so this crate writes them on demand: [`Fixture`] builds a
//! whole site folder, the same bytes on every machine, and `scripts/perf.sh`
//! measures Khalid against it.

mod fixture;
mod fixture_error;
mod inputs;
mod prepare_error;
mod site;
mod words;

pub use fixture::Fixture;
pub use fixture_error::FixtureError;
pub use inputs::{Page, Prepared};
pub use prepare_error::PrepareError;
