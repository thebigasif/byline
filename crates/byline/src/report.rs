//! Writes results and messages to the terminal.
//!
//! In human output, results go to standard output and messages to standard
//! error. With `--json`, everything goes to standard output as one JSON value,
//! so a script reads one stream and checks the exit code.

use std::io::{self, Write};

use byline_core::UserMessage;
use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;

/// The JSON shape of a message: `{"error": {"what": ..., "why": ..., "fix": ...}}`.
#[derive(Serialize)]
struct ErrorBody<'a> {
    error: &'a UserMessage,
}

/// The JSON shape of `--version --json`.
#[derive(Serialize)]
struct VersionBody {
    name: &'static str,
    version: &'static str,
}

/// Prints `text` to standard output and ends with `exit`.
pub(crate) fn text(text: &str, exit: Exit) -> Exit {
    finish(write!(io::stdout().lock(), "{text}"), exit)
}

/// Prints the version, as text or JSON.
pub(crate) fn version(output: Output) -> Exit {
    let version = env!("CARGO_PKG_VERSION");
    let written = match output {
        Output::Human => writeln!(io::stdout().lock(), "byline {version}"),
        Output::Json => json_line(&VersionBody {
            name: "byline",
            version,
        }),
    };
    finish(written, Exit::Success)
}

/// Prints `message` where the output mode says, and ends with `exit`.
pub(crate) fn failure(output: Output, message: &UserMessage, exit: Exit) -> Exit {
    let written = match output {
        Output::Human => writeln!(io::stderr().lock(), "{message}"),
        Output::Json => json_line(&ErrorBody { error: message }),
    };
    finish(written, exit)
}

fn json_line(body: &impl Serialize) -> io::Result<()> {
    let mut out = io::stdout().lock();
    serde_json::to_writer(&mut out, body)?;
    writeln!(out)
}

/// A terminal that cannot be written to leaves nothing to report the problem
/// on, so the exit code carries it alone.
fn finish(written: io::Result<()>, exit: Exit) -> Exit {
    written.map_or(Exit::Internal, |()| exit)
}
