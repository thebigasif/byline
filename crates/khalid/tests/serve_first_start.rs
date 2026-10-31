//! `khalid serve --init`: a host that starts Khalid on an empty disk, such
//! as Fly.io, Railway or Coolify, gets a new site at the address in
//! `KHALID_ADDRESS` on the first start, and the same site after that
//! (task 5.14, KH-INS-1, docs/plan.md 4.7).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::Value;

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

/// `serve --init` on `folder`, with `KHALID_ADDRESS` set to `address` or unset.
#[cfg(test)]
fn serve_init(folder: &Path, address: Option<&str>) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_khalid"));
    command
        .arg("--site")
        .arg(folder)
        .args(["serve", "--init", "--listen", "127.0.0.1:0"]);
    match address {
        Some(address) => command.env("KHALID_ADDRESS", address),
        None => command.env_remove("KHALID_ADDRESS"),
    };
    command
}

/// Starts the server and waits for the line that names its address.
#[cfg(test)]
fn start(mut command: Command) -> Server {
    let mut child = command
        .arg("--json")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line:?}"));
    let address = json["address"].as_str().unwrap();
    let rest = address.strip_prefix("http://").unwrap();
    let host = rest.split_once('/').unwrap().0.to_owned();
    Server { child, host }
}

/// The status of `GET /`.
#[cfg(test)]
fn home_status(server: &Server) -> u16 {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let request = format!("GET / HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    raw.split(' ').nth(1).unwrap().parse().unwrap()
}

#[cfg(test)]
fn settings(folder: &Path) -> PathBuf {
    let mut path = folder.to_path_buf();
    path.push("site.toml");
    path
}

#[test]
fn the_first_start_on_an_empty_disk_makes_the_site_at_the_given_address() {
    let disk = tempfile::tempdir().unwrap();
    let server = start(serve_init(disk.path(), Some("https://notes.example.org")));
    assert_eq!(home_status(&server), 200);
    let made = fs::read_to_string(settings(disk.path())).unwrap();
    assert!(
        made.contains("address = \"https://notes.example.org/\""),
        "{made}"
    );
    let mut posts = disk.path().to_path_buf();
    posts.push("content/en/posts");
    assert!(posts.is_dir());
}

#[test]
fn a_later_start_keeps_the_site_it_finds() {
    let disk = tempfile::tempdir().unwrap();
    drop(start(serve_init(
        disk.path(),
        Some("https://notes.example.org"),
    )));
    let first = fs::read_to_string(settings(disk.path())).unwrap();
    let server = start(serve_init(disk.path(), Some("https://moved.example.org")));
    assert_eq!(home_status(&server), 200);
    assert_eq!(fs::read_to_string(settings(disk.path())).unwrap(), first);
}

#[test]
fn an_empty_disk_with_no_address_stops_and_names_the_variable() {
    let disk = tempfile::tempdir().unwrap();
    let output = serve_init(disk.path(), None).output().unwrap();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(said.contains("KHALID_ADDRESS"), "{said}");
    assert_eq!(said.lines().count(), 3, "want what, why, fix:\n{said}");
    assert!(!settings(disk.path()).exists());
}

#[test]
fn serve_without_init_still_refuses_an_empty_folder() {
    let disk = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .arg("--site")
        .arg(disk.path())
        .args(["serve", "--listen", "127.0.0.1:0"])
        .env("KHALID_ADDRESS", "https://notes.example.org")
        .output()
        .unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(!settings(disk.path()).exists());
}
