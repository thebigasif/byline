//! `khalid-perf`: writes the generated sites `scripts/perf.sh` measures, and
//! times post page renders over one.
//!
//! ```text
//! khalid-perf fixture 1000 target/perf/site-1k
//! khalid-perf render target/perf/site-1k target/perf/render.json
//! khalid-perf lint target/perf/site-1k target/perf/lint.json
//! khalid-perf requests target/release/khalid 1000 target/perf/site-1k target/perf/requests.json
//! khalid-perf serve target/release/khalid 1000 target/perf/site-1k target/perf/serve-1k.json
//! ```

use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use khalid_core::SystemClock;
use khalid_lint::{LintConfig, Linter};
use khalid_perf::{
    Fixture, FixtureError, LintTimes, Prepared, RenderTimes, RequestRate, ServeTimes, Served,
};
use khalid_render::Renderer;

const USAGE: &str = "Usage: khalid-perf fixture POSTS FOLDER\n       \
                     khalid-perf render SITE OUT.json\n       \
                     khalid-perf lint SITE OUT.json\n       \
                     khalid-perf requests KHALID POSTS SITE OUT.json\n       \
                     khalid-perf serve KHALID POSTS SITE OUT.json";

/// Rounds over every page: one to warm up, then enough samples that the
/// 99th percentile of a 1,000-post site rests on 40 renders.
const ROUNDS: usize = 5;

/// Posts linted for the lint measure, spread over the site: as many
/// samples as the `divan` benchmark takes.
const LINT_SAMPLES: usize = 100;

/// Connections asking for the cached post page at once: `oha`'s default.
const CONNECTIONS: usize = 50;

/// How long the requests run, as the budget's `oha` run lasts.
const REQUESTS_FOR: Duration = Duration::from_secs(30);

/// Where Linux says when it gives a program transparent huge pages.
const THP_MODE: &str = "/sys/kernel/mm/transparent_hugepage/enabled";

/// Starts of `khalid serve`, and edits to one post, behind each median.
const SERVE_ROUNDS: usize = 5;

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
        ["lint", site, out] => lint(Path::new(site), Path::new(out)),
        ["requests", khalid, posts, site, out] => {
            requests(Path::new(khalid), posts, Path::new(site), Path::new(out))
        }
        ["serve", khalid, posts, site, out] => {
            serve(Path::new(khalid), posts, Path::new(site), Path::new(out))
        }
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

fn lint(site: &Path, out: &Path) -> Result<(), String> {
    let explain = |what: String| {
        format!(
            "{what}\nWithout it, the time to lint one post cannot be measured.\n\
             Write the site again with `khalid-perf fixture` and run this once more."
        )
    };
    let prepared = Prepared::read(site).map_err(|error| {
        explain(format!(
            "The site in {} could not be read: {error}.",
            site.display()
        ))
    })?;
    let linter = Linter::new(&LintConfig::default(), &SystemClock)
        .map_err(|error| explain(format!("The linter could not start: {error}.")))?;
    let times = LintTimes::measure(&prepared, &linter, LINT_SAMPLES, &SystemClock);
    fs::write(out, times.to_json())
        .map_err(|error| explain(format!("{} could not be written: {error}.", out.display())))
}

fn requests(khalid: &Path, posts: &str, site: &Path, out: &Path) -> Result<(), String> {
    let explain = |what: String| {
        format!(
            "{what}\nWithout it, the requests a second cannot be measured.\n\
             Build khalid with `cargo build --release -p khalid`, write the site \
             again with `khalid-perf fixture`, and run this once more."
        )
    };
    let fixture = posts.parse().ok().and_then(Fixture::new).ok_or_else(|| {
        format!("{USAGE}\nPOSTS is the number of posts the site was written with.")
    })?;
    let fail = |error| explain(format!("Serving {} failed: {error}.", site.display()));
    let (served, _) = Served::start(khalid, site, &SystemClock).map_err(fail)?;
    let address = fixture.post_address(0);
    served.get(&address).map_err(fail)?; // the page is cached before the run
    let rate = RequestRate::measure(
        served.host(),
        &address,
        (CONNECTIONS, REQUESTS_FOR),
        &SystemClock,
    )
    .map_err(fail)?;
    fs::write(out, rate.to_json())
        .map_err(|error| explain(format!("{} could not be written: {error}.", out.display())))
}

#[expect(
    clippy::print_stdout,
    reason = "the CI log shows where an idle server holds its memory"
)]
fn serve(khalid: &Path, posts: &str, site: &Path, out: &Path) -> Result<(), String> {
    let explain = |what: String| {
        format!(
            "{what}\nWithout it, the server's budgets cannot be measured.\n\
             Build khalid with `cargo build --release -p khalid`, write the site \
             again with `khalid-perf fixture`, and run this once more."
        )
    };
    let fixture = posts.parse().ok().and_then(Fixture::new).ok_or_else(|| {
        format!("{USAGE}\nPOSTS is the number of posts the site was written with.")
    })?;
    let times = ServeTimes::measure(khalid, (site, fixture), SERVE_ROUNDS, &SystemClock)
        .map_err(|error| explain(format!("Serving {} failed: {error}.", site.display())))?;
    if let Some(idle) = times.idle_memory() {
        print!("{posts} posts. {idle}");
    }
    // Whether the system hands out 2 MB pages unasked ("always") or only
    // where a program asks ("madvise") decides what the allocator can avoid.
    if let Ok(mode) = fs::read_to_string(THP_MODE) {
        println!("Transparent huge pages on this system: {}", mode.trim());
    }
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
