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

/// The server's resident memory each second of the idle wait, and its split
/// at the end.
#[derive(Debug, Clone, PartialEq)]
pub struct IdleMemory {
    each_second: Vec<f64>,
    split: Option<MemorySplit>,
}

impl IdleMemory {
    /// The readings `each_second`, in megabytes, the last at the end of the
    /// wait, and the `split` read then, when `/proc` gave one.
    pub fn new(each_second: Vec<f64>, split: Option<MemorySplit>) -> Self {
        Self { each_second, split }
    }

    /// The last reading, which is the idle memory measure.
    pub fn resident_mb(&self) -> Option<f64> {
        self.each_second.last().copied()
    }
}

impl fmt::Display for IdleMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let resident = self.resident_mb().unwrap_or(0.0);
        match self.split {
            Some(split) => writeln!(
                f,
                "Idle memory {resident:.2} MB: {:.2} MB anonymous, {:.2} MB of it in huge \
                 pages, and {:.2} MB of code and mapped files.",
                split.anonymous, split.huge_pages, split.files
            )?,
            None => writeln!(f, "Idle memory {resident:.2} MB.")?,
        }
        f.write_str("Resident memory each second after the first answer, in MB:")?;
        for reading in &self.each_second {
            write!(f, " {reading:.2}")?;
        }
        writeln!(f)
    }
}
