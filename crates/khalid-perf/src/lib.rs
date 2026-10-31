//! Generated large sites and the measures behind the performance budgets.
//!
//! The budgets in `docs/standards/performance.md` are measured on a site of
//! 1,000 posts and one of 10,000. Committing that many files would bury the
//! repository, so this crate writes them on demand: [`Fixture`] builds a
//! whole site folder, the same bytes on every machine, and `scripts/perf.sh`
//! measures Khalid against it.

mod fixture;
mod fixture_error;
mod idle_memory;
mod inputs;
mod lint_times;
mod prepare_error;
mod render_times;
mod request_rate;
mod serve_error;
mod serve_times;
mod served;
mod site;
mod words;

pub use fixture::Fixture;
pub use fixture_error::FixtureError;
pub use idle_memory::{AT_MOST, FLOOR, IdleMemory, MemorySplit, READINGS, STABLE, is_settled};
pub use inputs::{Page, Prepared};
pub use lint_times::LintTimes;
pub use prepare_error::PrepareError;
pub use render_times::{RenderTimes, percentile};
pub use request_rate::{RequestRate, read_response};
pub use serve_error::ServeError;
pub use serve_times::ServeTimes;
pub use served::{Served, announced, resident_mb};
