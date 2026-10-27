//! The files in `packaging/` that start Khalid on a server: each one's own
//! command line, run against the demo site, answers the home page (task
//! 5.14, KH-INS-1).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::Value;
use tempfile::TempDir;

/// A running `khalid serve`, stopped when dropped.
struct Server {
    child: Child,
    host: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The path of a file in the repository.
#[cfg(test)]
fn repo_file(relative: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../..");
    path.push(relative);
    path
}

/// A copy of the demo site, so the server's `data/` lands in a temp folder.
#[cfg(test)]
fn demo_copy() -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&repo_file("fixtures/demo-site"), temp.path());
    temp
}

#[cfg(test)]
fn copy_dir(from: &Path, into: &Path) {
    fs::create_dir_all(into).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let mut target = into.to_path_buf();
        target.push(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Runs a package's command line: `program` must be where the package puts
/// the binary, `site` and `listen` are swapped for the test's own, and
/// `--json` is added so the test can read the address the server took.
#[cfg(test)]
fn start(args: &[String], program: &str, site: (&str, &Path), listen: &str) -> Server {
    let (first, rest) = args.split_first().unwrap();
    assert_eq!(first, program, "the package runs Khalid from {first}");
    assert!(rest.iter().any(|arg| arg == site.0), "{rest:?}");
    assert!(rest.iter().any(|arg| arg == listen), "{rest:?}");
    let swapped = rest.iter().map(|arg| match arg.as_str() {
        arg if arg == site.0 => site.1.to_str().unwrap().to_owned(),
        arg if arg == listen => "127.0.0.1:0".to_owned(),
        arg => arg.to_owned(),
    });
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(swapped)
        .arg("--json")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    let stdout = child.stdout.take().unwrap();
    BufReader::new(stdout).read_line(&mut line).unwrap();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line}"));
    let address = json["address"].as_str().unwrap();
    let rest = address.strip_prefix("http://").unwrap();
    let host = rest.split_once('/').unwrap().0.to_owned();
    Server { child, host }
}

/// The status line and body of `GET path`.
#[cfg(test)]
fn get(server: &Server, path: &str) -> (u16, String) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    (
        head.split(' ').nth(1).unwrap().parse().unwrap(),
        body.to_owned(),
    )
}

/// The value of the one `key=` line in a systemd unit.
#[cfg(test)]
fn unit_value<'a>(unit: &'a str, key: &str) -> &'a str {
    let prefix = format!("{key}=");
    let mut found = unit.lines().filter_map(|line| line.strip_prefix(&prefix));
    let value = found.next().unwrap_or_else(|| panic!("no {key}= line"));
    assert!(found.next().is_none(), "more than one {key}= line");
    value
}

#[test]
fn the_systemd_unit_command_serves_the_demo_site() {
    let unit = fs::read_to_string(repo_file("packaging/systemd/khalid.service")).unwrap();
    let args: Vec<String> = unit_value(&unit, "ExecStart")
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let site = demo_copy();
    let server = start(
        &args,
        "/usr/local/bin/khalid",
        ("/var/lib/khalid", site.path()),
        "127.0.0.1:8080",
    );
    let (status, body) = get(&server, "/blog/");
    assert_eq!(status, 200);
    assert!(body.contains("Field Notes on SEO"), "{body}");
}

/// The steps in `docs/deploy.md`: `init` into the empty folder the guide
/// makes, then the unit's command serves that new site.
#[test]
fn a_site_made_as_the_deploy_guide_says_is_served_by_the_unit_command() {
    let unit = fs::read_to_string(repo_file("packaging/systemd/khalid.service")).unwrap();
    let guide = fs::read_to_string(repo_file("docs/deploy.md")).unwrap();
    let init = "khalid init /var/lib/khalid --address https://blog.example.com --yes";
    assert!(guide.contains(init), "the guide no longer says: {init}");
    let site = tempfile::tempdir().unwrap();
    let made = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["init", site.path().to_str().unwrap()])
        .args(["--address", "https://blog.example.com", "--yes"])
        .output()
        .unwrap();
    assert!(made.status.success(), "{made:?}");
    let args: Vec<String> = unit_value(&unit, "ExecStart")
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let server = start(
        &args,
        "/usr/local/bin/khalid",
        ("/var/lib/khalid", site.path()),
        "127.0.0.1:8080",
    );
    assert_eq!(get(&server, "/").0, 200);
}

#[test]
fn the_systemd_unit_keeps_the_service_in_its_site_folder() {
    let unit = fs::read_to_string(repo_file("packaging/systemd/khalid.service")).unwrap();
    let users = fs::read_to_string(repo_file("packaging/systemd/khalid.sysusers")).unwrap();
    assert_eq!(unit_value(&unit, "User"), "khalid");
    assert!(users.contains("u khalid - "), "{users}");
    assert_eq!(unit_value(&unit, "WorkingDirectory"), "/var/lib/khalid");
    assert_eq!(unit_value(&unit, "StateDirectory"), "khalid");
    assert_eq!(unit_value(&unit, "ProtectSystem"), "strict");
    assert_eq!(unit_value(&unit, "NoNewPrivileges"), "yes");
    assert_eq!(unit_value(&unit, "KillSignal"), "SIGTERM");
}

/// `systemd-analyze verify` reads the unit as systemd would. It runs where
/// systemd is installed and is skipped elsewhere, such as on macOS.
#[test]
fn the_systemd_unit_passes_systemd_analyze_verify() {
    let Ok(probe) = Command::new("systemd-analyze").arg("--version").output() else {
        eprintln!("systemd-analyze is not installed here, so the unit is not verified");
        return;
    };
    assert!(probe.status.success());
    let unit = fs::read_to_string(repo_file("packaging/systemd/khalid.service")).unwrap();
    let local = unit.replace("/usr/local/bin/khalid", env!("CARGO_BIN_EXE_khalid"));
    let folder = tempfile::tempdir().unwrap();
    let mut path = folder.path().to_path_buf();
    path.push("khalid.service");
    fs::write(&path, local).unwrap();
    let verify = Command::new("systemd-analyze")
        .args(["verify", "--man=no"])
        .arg(&path)
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&verify.stderr);
    assert!(verify.status.success(), "{said}");
    assert!(!said.contains("khalid.service"), "{said}");
}
