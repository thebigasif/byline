//! The budgets `khalid serve` answers for: cold start to first byte, idle
//! memory, and the rebuild after one edited post.

use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use khalid_core::Clock;

use crate::{Fixture, ServeError, Served, percentile};

/// How long the server rests after its warm-up requests before its memory
/// is read: longer than the half second between its looks at the folder.
const SETTLE: Duration = Duration::from_secs(2);

/// How many times the warm-up asks for each page.
const WARM_UP: usize = 10;

/// The pause between two edits, so each lands at a different point between
/// two looks at the folder.
const BETWEEN_EDITS: Duration = Duration::from_millis(173);

/// The medians of several starts and edits of `khalid serve`.
#[derive(Debug, Clone, PartialEq)]
pub struct ServeTimes {
    first_byte_ms: f64,
    idle_memory_mb: f64,
    rebuild_ms: f64,
}

impl ServeTimes {
    /// Measures already taken: the cold start to the first byte and the
    /// rebuild in milliseconds, and the idle memory in megabytes.
    pub fn new(first_byte_ms: f64, idle_memory_mb: f64, rebuild_ms: f64) -> Self {
        Self {
            first_byte_ms,
            idle_memory_mb,
            rebuild_ms,
        }
    }

    /// Starts `binary` on the generated `site` `rounds` times for the cold
    /// start, then, on the last start, reads the idle memory and edits its
    /// first post `rounds` times. The post is put back afterwards.
    ///
    /// # Errors
    ///
    /// The first [`ServeError`] a start, a request or an edit gives.
    pub fn measure(
        binary: &Path,
        (site, fixture): (&Path, Fixture),
        rounds: usize,
        clock: &dyn Clock,
    ) -> Result<Self, ServeError> {
        let mut starts = Vec::with_capacity(rounds);
        let mut last = None;
        for _ in 0..rounds.max(1) {
            drop(last.take()); // stops the previous server before the next starts
            let (served, ms) = Served::start(binary, site, clock)?;
            starts.push(ms);
            last = Some(served);
        }
        let served = last.ok_or(ServeError::NoAddress)?;
        let post = fixture.post_address(0);
        for _ in 0..WARM_UP {
            served.get(served.mount())?;
            served.get(&post)?;
        }
        thread::sleep(SETTLE);
        let idle_memory_mb = served.resident_mb()?;
        let rebuilds = edit_rounds(&served, site, fixture, rounds, clock)?;
        Ok(Self {
            first_byte_ms: median(starts),
            idle_memory_mb,
            rebuild_ms: median(rebuilds),
        })
    }

    /// The median cold start to the first byte of the home page.
    pub fn first_byte_ms(&self) -> f64 {
        self.first_byte_ms
    }

    /// The resident memory after the warm-up.
    pub fn idle_memory_mb(&self) -> f64 {
        self.idle_memory_mb
    }

    /// The median time from saving a post to its page showing the edit.
    pub fn rebuild_ms(&self) -> f64 {
        self.rebuild_ms
    }

    /// The three measures as the JSON object `scripts/perf.sh` merges into
    /// `perf/current.json`, with the names `perf/baseline.json` uses.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"cold_start_first_byte_ms\": {:.2}, \"idle_memory_mb\": {:.2}, \
             \"incremental_rebuild_ms\": {:.2}}}\n",
            self.first_byte_ms, self.idle_memory_mb, self.rebuild_ms
        )
    }
}

/// Edits the first post of `site` `rounds` times, each time timing until
/// its page shows the edit, then writes the post back as it was.
fn edit_rounds(
    served: &Served,
    site: &Path,
    fixture: Fixture,
    rounds: usize,
    clock: &dyn Clock,
) -> Result<Vec<f64>, ServeError> {
    let mut file = site.to_path_buf();
    file.push(fixture.post_path(0));
    let original = fixture.post_text(0);
    let address = fixture.post_address(0);
    let mut times = Vec::with_capacity(rounds);
    for round in 1..=rounds.max(1) {
        let marker = format!("Edited for the rebuild measure, round {round}.");
        let edited = format!("{original}\n{marker}\n");
        times.push(served.edit(&file, &edited, (&address, &marker), clock)?);
        thread::sleep(BETWEEN_EDITS);
    }
    fs::write(&file, original).map_err(|source| ServeError::Io { path: file, source })?;
    Ok(times)
}

/// The middle value of `values` by nearest rank.
fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    percentile(&values, 50)
}
