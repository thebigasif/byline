//! The plain sentences every dry-run report is made of.

use std::fmt;

/// `n` with the word that fits it.
pub(crate) fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// A line that counts `entries`, then lists them when there are any.
pub(crate) fn list(f: &mut fmt::Formatter<'_>, label: &str, entries: &[String]) -> fmt::Result {
    if entries.is_empty() {
        writeln!(f, "{label}: 0.")
    } else {
        writeln!(f, "{label}: {}.", entries.join("; "))
    }
}
