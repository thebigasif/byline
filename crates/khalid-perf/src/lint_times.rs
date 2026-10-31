//! How long linting one post against the whole site takes, in process.

use khalid_core::Clock;
use khalid_lint::Linter;

use crate::{Prepared, percentile};

/// The time to lint each of a sample of posts, sorted, in milliseconds.
#[derive(Debug, Clone, PartialEq)]
pub struct LintTimes {
    sorted_ms: Vec<f64>,
}

impl LintTimes {
    /// Times already taken, in milliseconds, in any order.
    pub fn from_ms(mut times: Vec<f64>) -> Self {
        times.sort_by(f64::total_cmp);
        Self { sorted_ms: times }
    }

    /// Lints `samples` posts of `site`, spread evenly from the first to the
    /// last, each against the whole site as `khalid publish` lints it, and
    /// times each with `clock`. One post costs tens of milliseconds on a
    /// 1,000-post site, so linting every post would take a minute. A site
    /// with fewer posts than `samples` has each post linted once.
    pub fn measure(site: &Prepared, linter: &Linter, samples: usize, clock: &dyn Clock) -> Self {
        let posts = &site.lint.posts;
        let step = (posts.len() / samples.max(1)).max(1);
        let mut times = Vec::with_capacity(samples);
        for post in posts.iter().step_by(step).take(samples) {
            let start = clock.now();
            let report = linter.lint_post(post, &site.lint);
            let nanos = (clock.now() - start).num_nanoseconds().unwrap_or(i64::MAX);
            drop(report); // dropped after the clock stops, so it is not timed
            times.push(f64::from(u32::try_from(nanos).unwrap_or(u32::MAX)) / 1e6);
        }
        Self::from_ms(times)
    }

    /// How many posts were timed.
    pub fn samples(&self) -> usize {
        self.sorted_ms.len()
    }

    /// The median time to lint one post.
    pub fn median_ms(&self) -> f64 {
        percentile(&self.sorted_ms, 50)
    }

    /// The measure as the JSON object `scripts/perf.sh` merges into
    /// `perf/current.json`, with the name `perf/baseline.json` uses.
    pub fn to_json(&self) -> String {
        format!("{{\"lint_one_post_ms\": {:.4}}}\n", self.median_ms())
    }
}
