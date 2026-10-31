//! `khalid update`: the binary replaced by a newer release, or with
//! `--rollback` put back to the version kept before the last update (plan
//! section 4.1). The version an update replaces is kept beside the binary,
//! with `.previous` after its name.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use khalid_core::UserMessage;
use semver::Version;
use serde::Serialize;

use crate::cli::{Cli, Output, UpdateTarget};
use crate::exit::Exit;
use crate::update_release::{self, Release};
use crate::{outbound, report, update_message};

/// What is added to the binary's name for the version kept beside it.
const KEPT: &str = ".previous";

/// What is added to the binary's name while two versions change places.
const ASIDE: &str = ".swapping";

/// The JSON shape of `khalid update --rollback --json`.
#[derive(Serialize)]
struct RollbackBody {
    binary: String,
    kept: String,
}

/// The JSON shape of `khalid update --json`.
#[derive(Serialize)]
struct UpdateBody {
    from: String,
    to: String,
    updated: bool,
    binary: String,
}

/// Runs `khalid update`, to the newest release or back to the kept version.
pub(crate) fn run(target: UpdateTarget, cli: &Cli) -> Exit {
    match target {
        UpdateTarget::Previous => rollback(cli.output),
        UpdateTarget::Latest => match latest(cli) {
            Ok(exit) => exit,
            Err(message) => report::failure(cli.output, &message, Exit::Failure),
        },
    }
}

/// Finds the newest release for this system and says whether it is newer
/// than the one running.
fn latest(cli: &Cli) -> Result<Exit, UserMessage> {
    let binary = running().map_err(|error| update_message::no_binary(&error))?;
    let platform = update_release::platform().ok_or_else(update_message::no_system)?;
    let current =
        Version::parse(env!("CARGO_PKG_VERSION")).map_err(|_| update_message::internal())?;
    let (Ok(client), Ok(runtime)) = (
        outbound::client(cli),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return Err(update_message::internal());
    };
    let release = runtime.block_on(update_release::newest(&client, releases(cli), platform))?;
    Ok(updated(cli.output, (&current, &release), &binary))
}

/// Says which version runs, and which is the newest.
fn updated(output: Output, (from, release): (&Version, &Release), binary: &Path) -> Exit {
    let (to, binary) = (release.version.to_string(), binary.display().to_string());
    let said = if release.version > *from {
        format!(
            "Khalid {to} is out, and this is {from}.\nDownload it from {}.\n",
            release.address(update_release::RELEASES)
        )
    } else {
        format!("Khalid {from} is the newest release, so nothing changed.\n")
    };
    match output {
        Output::Human => report::text(&said, Exit::Success),
        Output::Json => report::json(
            &UpdateBody {
                from: from.to_string(),
                updated: false,
                to,
                binary,
            },
            Exit::Success,
        ),
    }
}

/// Where releases are read from: the project's GitHub releases, or in a
/// test build the stand-in `--test-releases` names.
fn releases(cli: &Cli) -> &str {
    #[cfg(feature = "test-util")]
    if let Some(base) = &cli.test_releases {
        return base;
    }
    #[cfg(not(feature = "test-util"))]
    let _ = cli;
    update_release::RELEASES
}

/// Puts the binary back to the version kept before the last update, and
/// keeps the version that ran in its place, so a second rollback undoes
/// the first.
fn rollback(output: Output) -> Exit {
    let binary = match running() {
        Ok(binary) => binary,
        Err(error) => {
            let message = update_message::no_binary(&error);
            return report::failure(output, &message, Exit::Failure);
        }
    };
    let kept = suffixed(&binary, KEPT);
    if !kept.is_file() {
        let message = update_message::nothing_kept(&binary);
        return report::failure(output, &message, Exit::Failure);
    }
    if let Err(error) = swap(&binary, &kept) {
        return report::failure(
            output,
            &update_message::swap(&binary, &error),
            Exit::Failure,
        );
    }
    let (binary, kept) = (binary.display().to_string(), kept.display().to_string());
    match output {
        Output::Human => report::text(
            &format!(
                "Khalid went back to the version kept before the last update, in {binary}.\nThe version you ran is kept in {kept}, so `khalid update --rollback` undoes this.\n"
            ),
            Exit::Success,
        ),
        Output::Json => report::json(&RollbackBody { binary, kept }, Exit::Success),
    }
}

/// The file this process runs from, with links followed, so an update
/// replaces the real file and not a link to it.
fn running() -> io::Result<PathBuf> {
    std::env::current_exe().and_then(fs::canonicalize)
}

/// `path` with `suffix` added to its file name.
fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(suffix);
    PathBuf::from(name)
}

/// Swaps the files at `binary` and `kept`. When the second move fails, the
/// first is undone, so `binary` always holds a version.
fn swap(binary: &Path, kept: &Path) -> io::Result<()> {
    let aside = suffixed(binary, ASIDE);
    fs::rename(binary, &aside)?;
    if let Err(error) = fs::rename(kept, binary) {
        // Best effort: the error that matters is the one returned.
        drop(fs::rename(&aside, binary));
        return Err(error);
    }
    fs::rename(&aside, kept)
}
