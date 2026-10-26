//! `khalid-perf`: writes the generated sites `scripts/perf.sh` measures, and
//! times post page renders over one.
//!
//! ```text
//! khalid-perf fixture 1000 target/perf/site-1k
//! khalid-perf render target/perf/site-1k target/perf/render.json
//! ```

use std::fs;
use std::path::Path;
use std::process::ExitCode;

use khalid_core::SystemClock;
use khalid_perf::{Fixture, FixtureError, Prepared, RenderTimes};
use khalid_render::Renderer;

const USAGE: &str = "Usage: khalid-perf fixture POSTS FOLDER\n       \
                     khalid-perf render SITE OUT.json";

/// Rounds over every page: one to warm up, then enough samples that the
/// 99th percentile of a 1,000-post site rests on 40 renders.
const ROUNDS: usize = 5;

#[expect(
    clippy::print_stderr,
    reason = "a developer tool says what went wrong on standard error"
)]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match *args.as_slice() {
        ["fixture", posts, folder] => fixture(posts, Path::new(folder)),
        ["render", site, out] => render(Path::new(site), Path::new(out)),
        _ => Err(USAGE.to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn fixture(posts: &str, folder: &Path) -> Result<(), String> {
    let fixture = posts
        .parse()
        .ok()
        .and_then(Fixture::new)
        .ok_or_else(|| format!("{USAGE}\nPOSTS is a whole number above zero."))?;
    fixture.write(folder).map_err(|error| message(&error))
}

fn render(site: &Path, out: &Path) -> Result<(), String> {
    let explain = |what: String| {
        format!(
            "{what}\nWithout it, render times cannot be measured.\n\
             Write the site again with `khalid-perf fixture` and run this once more."
        )
    };
    let prepared = Prepared::read(site).map_err(|error| {
        explain(format!(
            "The site in {} could not be read: {error}.",
            site.display()
        ))
    })?;
    let renderer =
        Renderer::new().map_err(|error| explain(format!("The templates failed: {error}.")))?;
    let times = RenderTimes::measure(&prepared, &renderer, ROUNDS, &SystemClock)
        .map_err(|error| explain(format!("A page failed to render: {error}.")))?;
    fs::write(out, times.to_json())
        .map_err(|error| explain(format!("{} could not be written: {error}.", out.display())))
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
