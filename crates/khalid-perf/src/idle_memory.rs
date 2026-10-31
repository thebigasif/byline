//! Where an idle `khalid serve` holds its memory, and how the memory moved
//! while it waited, printed beside the idle memory measure so that a
//! failing budget can be read from the CI log.

use std::fmt;

/// The parts of a process's resident memory, from `/proc/<pid>/status`
/// and `/proc/<pid>/smaps_rollup`, in megabytes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemorySplit {
    anonymous: f64,
    files: f64,
    huge_pages: f64,
}

impl MemorySplit {
    /// The split from the text of the `status` and `smaps_rollup` files.
    /// `None` when `status` lacks `RssAnon` or `RssFile`; a rollup without
    /// `AnonHugePages` counts no huge pages.
    pub fn parse(status: &str, rollup: &str) -> Option<Self> {
        Some(Self {
            anonymous: field_mb(status, "RssAnon:")?,
            files: field_mb(status, "RssFile:")?,
            huge_pages: field_mb(rollup, "AnonHugePages:").unwrap_or(0.0),
        })
    }

    /// Memory the process allocated: its heaps, stacks and allocator arenas.
    pub fn anonymous_mb(&self) -> f64 {
        self.anonymous
    }

    /// Pages of the binary, its libraries and other mapped files.
    pub fn files_mb(&self) -> f64 {
        self.files
    }

    /// The part of the anonymous memory held in 2 MB huge pages, which
    /// count whole however little of them is used.
    pub fn huge_pages_mb(&self) -> f64 {
        self.huge_pages
    }
}

/// The value in megabytes of the line starting with `name` in a `/proc`
/// file, whose values are in kilobytes.
pub(crate) fn field_mb(text: &str, name: &str) -> Option<f64> {
    let line = text.lines().find(|line| line.starts_with(name))?;
    let kilobytes: u32 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(f64::from(kilobytes) / 1024.0)
}

/// How many readings, one second apart, are taken once the server has
/// settled. The measure is the lowest of them, so a page the allocator
/// hands back between two readings cannot raise it.
pub const READINGS: usize = 5;

/// The seconds the thread count and the resident reading must both hold
/// still before the server counts as settled. Tokio's blocking threads
/// end ten seconds after their last task and resident memory falls with
/// them, so while either moves the server has not settled.
pub const STABLE: usize = 3;

/// The first second of the wait that can settle it. The wait must
/// outlast tokio's blocking threads, which end ten seconds after their
/// last task, so a plateau before they end never passes for settled.
pub const FLOOR: usize = 15;

/// The most seconds the settle wait may run, after which the readings
/// are taken anyway, so a wedged server cannot hang the measure.
pub const AT_MOST: usize = 60;

/// The most the resident reading may fall between two seconds and still
/// count as holding still, in megabytes. A fall larger than this is the
/// allocator handing pages back, not jitter.
const DRIFT: f64 = 0.5;

/// The server's resident memory each second of the idle wait, where the
/// wait stopped waiting for the server to settle, and the split at the
/// end.
#[derive(Debug, Clone, PartialEq)]
pub struct IdleMemory {
    each_second: Vec<f64>,
    settled: usize,
    split: Option<MemorySplit>,
}

impl IdleMemory {
    /// The readings `each_second`, in megabytes, where the last
    /// [`READINGS`] of them were taken after the server settled at
    /// `settled`, and the `split` read at the end, when `/proc` gave one.
    pub fn settled_at(each_second: Vec<f64>, settled: usize, split: Option<MemorySplit>) -> Self {
        Self {
            each_second,
            settled,
            split,
        }
    }

    /// The idle memory measure: the lowest of the readings taken one
    /// second apart once the server had settled. The last reading alone
    /// can catch the middle of a fall.
    pub fn resident_mb(&self) -> Option<f64> {
        self.each_second
            .get(self.settled..)?
            .iter()
            .copied()
            .min_by(f64::total_cmp)
    }
}

/// Whether the server has settled given `trace`, the thread count and
/// resident megabytes read each second since the first answer. It has
/// when the trace is past [`FLOOR`] seconds and the last [`STABLE`]
/// readings show the thread count unchanged and the resident reading not
/// falling by more than a drift. A server whose platform does not say
/// its thread count settles on its memory alone.
pub fn is_settled(trace: &[(Option<usize>, f64)]) -> bool {
    let Some(tail) = trace.get(trace.len().saturating_sub(STABLE)..) else {
        return false;
    };
    if trace.len() < FLOOR {
        return false;
    }
    tail.windows(2).all(|pair| {
        let (Some(before_threads), Some(after_threads)) = (pair.first(), pair.get(1)) else {
            return false;
        };
        before_threads.0 == after_threads.0 && after_threads.1 >= before_threads.1 - DRIFT
    })
}

impl fmt::Display for IdleMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let resident = self.resident_mb().unwrap_or(0.0);
        match self.split {
            Some(split) => writeln!(
                f,
                "Idle memory {resident:.2} MB, the lowest of the readings after second \
                 {}: {:.2} MB anonymous, {:.2} MB of it in huge pages, and {:.2} MB of \
                 code and mapped files.",
                self.settled + 1,
                split.anonymous,
                split.huge_pages,
                split.files
            )?,
            None => writeln!(
                f,
                "Idle memory {resident:.2} MB, the lowest of the readings after second {}.",
                self.settled + 1
            )?,
        }
        f.write_str("Resident memory each second after the first answer, in MB:")?;
        for reading in &self.each_second {
            write!(f, " {reading:.2}")?;
        }
        writeln!(f)
    }
}
