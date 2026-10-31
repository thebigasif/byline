//! `khalid update`: the binary replaced by a newer release, or with
//! `--rollback` put back to the version kept before the last update (plan
//! section 4.1). The version an update replaces is kept beside the binary,
//! with `.previous` after its name.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::cli::Output;
use crate::exit::Exit;
use crate::{report, update_message};

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

/// Puts the binary back to the version kept before the last update, and
/// keeps the version that ran in its place, so a second rollback undoes
/// the first.
pub(crate) fn rollback(output: Output) -> Exit {
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
pub(crate) fn running() -> io::Result<PathBuf> {
    std::env::current_exe().and_then(fs::canonicalize)
}

/// `path` with `suffix` added to its file name.
pub(crate) fn suffixed(path: &Path, suffix: &str) -> PathBuf {
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
