//! `khalid-perf`: writes the generated sites `scripts/perf.sh` measures.
//!
//! ```text
//! khalid-perf fixture 1000 target/perf/site-1k
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use khalid_perf::{Fixture, FixtureError};

const USAGE: &str = "Usage: khalid-perf fixture POSTS FOLDER";

#[expect(
    clippy::print_stderr,
    reason = "a developer tool says what went wrong on standard error"
)]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [command, posts, folder] = args.as_slice() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let fixture = posts.parse().ok().and_then(Fixture::new);
    let (Some(fixture), "fixture") = (fixture, command.as_str()) else {
        eprintln!("{USAGE}\nPOSTS is a whole number above zero.");
        return ExitCode::from(2);
    };
    match fixture.write(&PathBuf::from(folder)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", message(&error));
            ExitCode::FAILURE
        }
    }
}

/// What happened, why it matters and what to do, for a developer.
fn message(error: &FixtureError) -> String {
    match error {
        FixtureError::NotEmpty { path } => format!(
            "The folder {} already holds files.\n\
             A generated site would mix with them and the measures would be wrong.\n\
             Delete the folder or pick an empty one.",
            path.display()
        ),
        other => format!(
            "Khalid could not write the generated site: {other}.\n\
             Without it, nothing can be measured.\n\
             Check the disk has space and the folder can be written to."
        ),
    }
}
