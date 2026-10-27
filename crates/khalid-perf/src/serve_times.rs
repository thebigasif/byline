//! The budgets `khalid serve` answers for: cold start to first byte, idle
//! memory, and the rebuild after one edited post.

use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use khalid_core::Clock;

use crate::{Fixture, IdleMemory, ServeError, Served, percentile};

/// How long after its first answer, with no traffic in between, the
/// server's memory counts as idle (CTO answers 2026-10-27, section 2).
const IDLE: Duration = Duration::from_secs(15);

/// The pause between two edits, short enough that an edit can land while
/// the full render after the one before still runs.
const BETWEEN_EDITS: Duration = Duration::from_millis(173);

/// The medians of several starts and edits of `khalid serve`.
#[derive(Debug, Clone, PartialEq)]
pub struct ServeTimes {
    first_byte_ms: f64,
    idle_memory_mb: f64,
    rebuild_ms: f64,
    idle: Option<IdleMemory>,
}

impl ServeTimes {
    /// Measures already taken: the cold start to the first byte and the
    /// rebuild in milliseconds, and the idle memory in megabytes.
    pub fn new(first_byte_ms: f64, idle_memory_mb: f64, rebuild_ms: f64) -> Self {
        Self {
            first_byte_ms,
            idle_memory_mb,
            rebuild_ms,
            idle: None,
        }
    }

    /// Starts `binary` on the generated `site` `rounds` times for the cold
    /// start, then, on the last start, reads the idle memory 15 seconds after
    /// its first answer, with no request in between, and edits its first
    /// post `rounds` times. The post is put back afterwards.
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
        let idle = idle_memory(&served)?;
        let rebuilds = edit_rounds(&served, site, fixture, rounds, clock)?;
        Ok(Self {
            first_byte_ms: median(starts),
            idle_memory_mb: idle.resident_mb().ok_or(ServeError::NoMemoryReport)?,
            rebuild_ms: median(rebuilds),
            idle: Some(idle),
        })
    }

    /// The memory readings behind the idle measure, when it was measured
    /// here rather than given to [`ServeTimes::new`].
    pub fn idle_memory(&self) -> Option<&IdleMemory> {
        self.idle.as_ref()
    }

    /// The median cold start to the first byte of the home page.
    pub fn first_byte_ms(&self) -> f64 {
        self.first_byte_ms
    }

    /// The resident memory 15 seconds after the first answer.
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

/// Reads the resident memory of `served` each second for [`IDLE`] after
/// its first answer, which was the start's first byte, with no request in
/// between. Reading `/proc` from here touches nothing in the server.
fn idle_memory(served: &Served) -> Result<IdleMemory, ServeError> {
    let mut each_second = Vec::new();
    for _ in 0..IDLE.as_secs() {
        thread::sleep(Duration::from_secs(1));
        each_second.push(served.resident_mb()?);
    }
    Ok(IdleMemory::new(each_second, served.memory_split()))
}

/// The middle value of `values` by nearest rank.
fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    percentile(&values, 50)
}
