//! Where an old address leads, the same for every source, and the CSV the
//! host or the redirect manager reads.

use std::fmt::Write as _;

/// How long a move lasts, which search engines read from the status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MoveStatus {
    /// 301, or the plugin's 308: the page moved for good.
    Permanent,
    /// 302, or the plugin's 303: the page is elsewhere for now.
    Found,
    /// 307: the page is elsewhere for now.
    Temporary,
}

/// How an entry matches old addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Matching {
    /// One path, exactly.
    Exact,
    /// A regular expression over the path, from a redirect plugin.
    Pattern,
}

/// Where an old address leads.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MapTarget {
    /// Nowhere new: the address is the same in Khalid.
    Kept,
    /// To a new address, a full one or a path the plugin wrote.
    Moved {
        /// Where the visitor goes.
        to: String,
        /// How long the move lasts.
        status: MoveStatus,
    },
    /// 410: gone for good.
    Gone,
    /// 451: withheld for legal reasons.
    Unavailable,
}

/// A permanent move to `to`.
pub(crate) fn moved(to: &str) -> MapTarget {
    MapTarget::Moved {
        to: to.to_owned(),
        status: MoveStatus::Permanent,
    }
}

/// The map as CSV for the host or the redirect manager: every entry that
/// redirects, with its status and whether it matches a path or a pattern.
/// Each of `rows` is an old address, where it leads and how it matches.
pub(crate) fn redirects_csv<'a>(
    rows: impl IntoIterator<Item = (&'a str, &'a MapTarget, Matching)>,
) -> String {
    let mut csv = String::from("from,to,status,match\n");
    for (from, to, matching) in rows {
        let (to, status) = match to {
            MapTarget::Moved { to, status } => (
                to.as_str(),
                match status {
                    MoveStatus::Found => "302",
                    MoveStatus::Temporary => "307",
                    MoveStatus::Permanent => "301",
                },
            ),
            MapTarget::Gone => ("", "410"),
            MapTarget::Unavailable => ("", "451"),
            MapTarget::Kept => continue,
        };
        let matching = if matching == Matching::Pattern {
            "regex"
        } else {
            "exact"
        };
        let (from, to) = (field(from), field(to));
        // Writing to a String cannot fail.
        let _ = writeln!(csv, "{from},{to},{status},{matching}");
    }
    csv
}

/// A CSV field, quoted when it holds a comma, a quote or a line break.
fn field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}
