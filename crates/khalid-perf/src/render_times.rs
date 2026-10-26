//! How long each post page takes to render, in process.

use khalid_core::Clock;
use khalid_render::{RenderError, Renderer};

use crate::Prepared;

/// The render time of every post page over several rounds, sorted, in
/// milliseconds. The first round warms caches and is not kept.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderTimes {
    sorted_ms: Vec<f64>,
}

impl RenderTimes {
    /// Renders every page of `site` `rounds` times, timing each render with
    /// `clock`, and keeps every round after the first.
    ///
    /// # Errors
    ///
    /// The first [`RenderError`] a page gives.
    pub fn measure(
        site: &Prepared,
        renderer: &Renderer,
        rounds: usize,
        clock: &dyn Clock,
    ) -> Result<Self, RenderError> {
        let kept = rounds.saturating_sub(1) * site.pages.len();
        let mut times = Vec::with_capacity(kept);
        for round in 0..rounds {
            for page in &site.pages {
                let start = clock.now();
                renderer.render(&site.site, &page.meta, &page.body)?;
                let nanos = (clock.now() - start).num_nanoseconds().unwrap_or(i64::MAX);
                if round > 0 {
                    times.push(f64::from(u32::try_from(nanos).unwrap_or(u32::MAX)) / 1e6);
                }
            }
        }
        times.sort_by(f64::total_cmp);
        Ok(Self { sorted_ms: times })
    }

    /// How many renders were kept.
    pub fn samples(&self) -> usize {
        self.sorted_ms.len()
    }

    /// The median render time.
    pub fn p50_ms(&self) -> f64 {
        percentile(&self.sorted_ms, 50)
    }

    /// The render time 99 in 100 pages stay under.
    pub fn p99_ms(&self) -> f64 {
        percentile(&self.sorted_ms, 99)
    }

    /// The two measures as the JSON object `scripts/perf.sh` merges into
    /// `perf/current.json`, with the names `perf/baseline.json` uses.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"post_render_p50_ms\": {:.4}, \"post_render_p99_ms\": {:.4}}}\n",
            self.p50_ms(),
            self.p99_ms()
        )
    }
}

/// The `percent` percentile of `sorted` by nearest rank, or 0 when empty.
pub fn percentile(sorted: &[f64], percent: usize) -> f64 {
    let rank = (sorted.len() * percent).div_ceil(100).max(1);
    sorted.get(rank - 1).copied().unwrap_or(0.0)
}
