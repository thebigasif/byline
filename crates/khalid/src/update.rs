//! `khalid update`: the binary replaced by a newer release, or with
//! `--rollback` put back to the version kept before the last update (plan
//! section 4.1). The version an update replaces is kept beside the binary,
//! with `.previous` after its name.

use std::ffi::OsString;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::Duration;

use khalid_core::UserMessage;
use khalid_outbound::Limits;
use semver::Version;
use serde::Serialize;
use tempfile::NamedTempFile;

use crate::cli::{Cli, Output, UpdateTarget};
use crate::exit::Exit;
use crate::update_release::{self, Release};
use crate::{outbound, report, update_message};

/// What is added to the binary's name for the version kept beside it.
const KEPT: &str = ".previous";

/// What is added to the binary's name while two versions change places.
const ASIDE: &str = ".swapping";

/// The largest release file Khalid fetches, well above the 30 MB binary
/// budget in docs/standards/performance.md.
const LARGEST: usize = 200 * 1024 * 1024;

/// How long fetching one release file may take, for a slow connection.
const PATIENCE: Duration = Duration::from_secs(300);

/// The JSON shape of `khalid update --rollback --json`.
#[derive(Serialize)]
struct RollbackBody {
    binary: String,
    kept: String,
}

/// The JSON shape of `khalid update --json`. `kept` is there when the
/// binary was replaced.
#[derive(Serialize)]
struct UpdateBody {
    from: String,
    to: String,
    updated: bool,
    binary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kept: Option<String>,
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

/// Puts the newest release in place of the binary when it is newer than
/// the one running, after checking it against `SHA256SUMS`, and keeps the
/// old version beside it.
fn latest(cli: &Cli) -> Result<Exit, UserMessage> {
    let binary = running().map_err(|error| update_message::no_binary(&error))?;
    let platform = update_release::platform().ok_or_else(update_message::no_system)?;
    let current =
        Version::parse(env!("CARGO_PKG_VERSION")).map_err(|_| update_message::internal())?;
    let limits = Limits::default()
        .with_max_body_bytes(LARGEST)
        .with_timeout(PATIENCE);
    let (Ok(client), Ok(runtime)) = (
        outbound::client_within(cli, limits),
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build(),
    ) else {
        return Err(update_message::internal());
    };
    let base = releases(cli);
    let fetched = runtime.block_on(async {
        let release = update_release::newest(&client, base, platform).await?;
        if release.version <= current {
            return Ok((release, None));
        }
        let bytes = update_release::download(&client, base, &release).await?;
        Ok::<_, UserMessage>((release, Some(bytes)))
    })?;
    let kept = match fetched {
        (_, Some(bytes)) => {
            let kept =
                install(&binary, &bytes).map_err(|error| update_message::swap(&binary, &error))?;
            Some(kept.display().to_string())
        }
        (_, None) => None,
    };
    Ok(updated(cli.output, (&current, &fetched.0), &binary, kept))
}

/// Says which version runs now, and where the old one is kept when the
/// binary was replaced.
fn updated(
    output: Output,
    (from, release): (&Version, &Release),
    binary: &Path,
    kept: Option<String>,
) -> Exit {
    let (to, binary) = (release.version.to_string(), binary.display().to_string());
    let said = match &kept {
        Some(kept) => format!(
            "Updated Khalid from {from} to {to} in {binary}.\nThe old version is kept in {kept}, and `khalid update --rollback` goes back to it.\n"
        ),
        None => format!("Khalid {from} is the newest release, so nothing changed.\n"),
    };
    match output {
        Output::Human => report::text(&said, Exit::Success),
        Output::Json => report::json(
            &UpdateBody {
                from: from.to_string(),
                updated: kept.is_some(),
                to,
                binary,
                kept,
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

/// Writes `bytes` beside `binary` with its permissions, moves `binary` to
/// the kept name and the new file into its place, and returns the kept
/// file. When the last move fails, the old version goes back.
fn install(binary: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
    let folder = binary
        .parent()
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    let mut fresh = NamedTempFile::new_in(folder)?;
    fresh.write_all(bytes)?;
    fresh
        .as_file()
        .set_permissions(fs::metadata(binary)?.permissions())?;
    fresh.as_file().sync_all()?;
    let kept = suffixed(binary, KEPT);
    if cfg!(windows) && kept.exists() {
        // Windows will not move a file over another.
        fs::remove_file(&kept)?;
    }
    fs::rename(binary, &kept)?;
    if let Err(error) = fresh.persist(binary) {
        drop(fs::rename(&kept, binary));
        return Err(error.error);
    }
    Ok(kept)
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
