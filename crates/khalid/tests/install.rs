//! `scripts/install.sh`: it installs the release file for this machine only
//! after its checksum matches, and refuses everything else with a message
//! that says what, why and what to do (task 5.14, KH-INS-1). The releases
//! come from a folder through `KHALID_RELEASES`, so no test touches the
//! network (RULES.md BL-TST-3).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

/// The release name every test installs.
const VERSION: &str = "v9.8.7";

/// What the stand-in binary prints, so a test can tell it ran.
const SAYS: &str = "khalid stand-in 9.8.7";

/// A folder laid out like GitHub's release downloads, and a folder to
/// install into.
struct Release {
    root: TempDir,
    into: TempDir,
}

/// The platform name the script picks on this machine.
#[cfg(test)]
fn platform() -> String {
    let os = match std::env::consts::OS {
        "macos" => "macos",
        _ => "linux",
    };
    format!("{os}-{}", std::env::consts::ARCH)
}

#[cfg(test)]
fn file_name() -> String {
    format!("khalid-{VERSION}-{}", platform())
}

/// The path of `name` in the release's folder.
#[cfg(test)]
fn in_release(release: &Release, name: &str) -> PathBuf {
    let mut path = release.root.path().to_path_buf();
    path.push(VERSION);
    path.push(name);
    path
}

/// The SHA-256 of a file, from the same tool the script uses.
#[cfg(test)]
fn sha256(path: &Path) -> String {
    let output = Command::new("sha256sum")
        .arg(path)
        .output()
        .or_else(|_| {
            Command::new("shasum")
                .args(["-a", "256"])
                .arg(path)
                .output()
        })
        .unwrap();
    let text = String::from_utf8(output.stdout).unwrap();
    text.split_whitespace().next().unwrap().to_owned()
}

/// A release holding a stand-in binary and a SHA256SUMS that matches it.
#[cfg(test)]
fn release() -> Release {
    let release = Release {
        root: tempfile::tempdir().unwrap(),
        into: tempfile::tempdir().unwrap(),
    };
    fs::create_dir_all(in_release(&release, "")).unwrap();
    let binary = in_release(&release, &file_name());
    fs::write(&binary, format!("#!/bin/sh\necho '{SAYS}'\n")).unwrap();
    let sums = format!(
        "{}  {}\n{}  khalid-{VERSION}-windows-x86_64.exe\n",
        sha256(&binary),
        file_name(),
        "0".repeat(64),
    );
    fs::write(in_release(&release, "SHA256SUMS"), sums).unwrap();
    release
}

/// Runs the script with `args`, reading releases from `release`.
#[cfg(test)]
fn install(release: &Release, args: &[&str]) -> Output {
    let mut script = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    script.push("../../scripts/install.sh");
    let source = format!("file://{}", release.root.path().display());
    Command::new("sh")
        .arg(script)
        .args(args)
        .env("KHALID_RELEASES", source)
        .output()
        .unwrap()
}

/// The folder the tests install into, as an argument.
#[cfg(test)]
fn into(release: &Release) -> &str {
    release.into.path().to_str().unwrap()
}

#[cfg(test)]
fn installed(release: &Release) -> PathBuf {
    let mut path = release.into.path().to_path_buf();
    path.push("khalid");
    path
}

/// The refusal starts the same way every time and gives a fix on the next
/// line. Before it may stand curl's own reason, such as a missing file.
#[cfg(test)]
fn assert_refused(output: &Output, why: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    let mut lines = stderr
        .lines()
        .skip_while(|line| !line.starts_with("Khalid was not installed. "));
    let what = lines.next().unwrap_or_else(|| panic!("{stderr}"));
    assert!(what.contains(why), "{stderr}");
    assert!(lines.next().is_some(), "no fix given: {stderr}");
}

#[test]
fn the_script_installs_the_checked_binary_and_it_runs() {
    let release = release();
    let output = install(&release, &["--version", VERSION, "--dir", into(&release)]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{output:?}");
    assert!(
        stdout.contains(&format!("Installed Khalid {VERSION} at ")),
        "{stdout}"
    );
    let ran = Command::new(installed(&release)).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), SAYS);
}

#[test]
fn the_script_replaces_an_older_binary() {
    let release = release();
    fs::write(installed(&release), "#!/bin/sh\necho old\n").unwrap();
    let output = install(&release, &["--version", VERSION, "--dir", into(&release)]);
    assert!(output.status.success(), "{output:?}");
    let ran = Command::new(installed(&release)).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&ran.stdout).trim(), SAYS);
}

#[test]
fn a_download_that_does_not_match_its_checksum_is_not_installed() {
    let release = release();
    fs::write(
        in_release(&release, &file_name()),
        "#!/bin/sh\necho changed\n",
    )
    .unwrap();
    let output = install(&release, &["--version", VERSION, "--dir", into(&release)]);
    assert_refused(&output, "does not match its checksum");
    assert!(!installed(&release).exists());
}

#[test]
fn a_release_with_no_checksum_for_the_file_is_not_installed() {
    let release = release();
    fs::write(in_release(&release, "SHA256SUMS"), "").unwrap();
    let output = install(&release, &["--version", VERSION, "--dir", into(&release)]);
    assert_refused(&output, "SHA256SUMS has no line for");
    assert!(!installed(&release).exists());
}

#[test]
fn a_release_without_the_file_says_it_could_not_be_downloaded() {
    let release = release();
    fs::remove_file(in_release(&release, &file_name())).unwrap();
    let output = install(&release, &["--version", VERSION, "--dir", into(&release)]);
    assert_refused(&output, "could not be downloaded");
}

#[test]
fn a_name_that_is_not_a_release_is_refused_before_any_download() {
    let release = release();
    let output = install(&release, &["--version", "latest", "--dir", into(&release)]);
    assert_refused(&output, "latest is not a release name");
}

#[test]
fn an_unknown_option_is_refused() {
    let release = release();
    let output = install(&release, &["--prefix", "/opt"]);
    assert_refused(&output, "--prefix is not one this script knows");
}

#[test]
fn the_script_is_plain_posix_sh() {
    let mut script = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    script.push("../../scripts/install.sh");
    let text = fs::read_to_string(&script).unwrap();
    assert!(text.starts_with("#!/bin/sh\n"));
    let parsed = Command::new("sh").arg("-n").arg(&script).output().unwrap();
    assert!(parsed.status.success(), "{parsed:?}");
}
