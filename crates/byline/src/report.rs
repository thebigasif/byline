//! Writes results and messages to the terminal: results to standard output,
//! messages to standard error.

use std::io::{self, Write};

use byline_core::UserMessage;

use crate::exit::Exit;

/// Prints `text` to standard output and ends with `exit`.
pub(crate) fn text(text: &str, exit: Exit) -> Exit {
    finish(write!(io::stdout().lock(), "{text}"), exit)
}

/// Prints `message` to standard error and ends with `exit`.
pub(crate) fn failure(message: &UserMessage, exit: Exit) -> Exit {
    finish(writeln!(io::stderr().lock(), "{message}"), exit)
}

/// A terminal that cannot be written to leaves nothing to report the problem
/// on, so the exit code carries it alone.
fn finish(written: io::Result<()>, exit: Exit) -> Exit {
    written.map_or(Exit::Internal, |()| exit)
}
