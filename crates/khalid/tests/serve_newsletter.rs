//! The newsletter on a running `khalid serve`, with scripts off: the plain
//! sign-up form gets a whole page back, at the mount path and where the
//! `::newsletter` block posts, and while the site cannot send email that
//! page says sign-ups are not open and nothing is stored (task 5.5).

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

/// A copy of the demo site, so the database it makes stays out of the
/// fixture.
#[cfg(test)]
fn demo_copy() -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
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

/// Starts `khalid serve` on `site` on a free port.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "serve", "--listen"])
        .args(["127.0.0.1:0", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    let stdout = child.stdout.take().unwrap();
    BufReader::new(stdout).read_line(&mut line).unwrap();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line}"));
    let address = json["address"].as_str().unwrap();
    let host = address.strip_prefix("http://").unwrap();
    let host = host.trim_end_matches("/blog/").to_owned();
    Server { child, host }
}

/// Posts the form `body` to `path` as a browser does, and returns the
/// status and the page.
#[cfg(test)]
fn post_form(server: &Server, path: &str, body: &str) -> (u16, String) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\
         Content-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, page) = raw.split_once("\r\n\r\n").unwrap();
    (
        head.split(' ').nth(1).unwrap().parse().unwrap(),
        page.to_owned(),
    )
}

#[test]
fn the_plain_form_gets_a_page_saying_sign_ups_are_not_open_yet() {
    let site = demo_copy();
    let server = start(site.path());
    for path in ["/blog/subscribe", "/subscribe"] {
        let (status, page) = post_form(&server, path, "email=grace%40example.com&homepage=");
        assert_eq!(status, 503, "{path}: {page}");
        assert!(
            page.contains("<h1>Signing up is not open yet</h1>"),
            "{page}"
        );
        assert!(page.contains("nothing was saved"), "{page}");
    }
    drop(server);
    let needle = b"grace@example.com";
    for name in ["site.db", "site.db-wal"] {
        let mut file = site.path().to_path_buf();
        file.extend(["data", name]);
        let bytes = fs::read(&file).unwrap_or_default();
        assert!(
            !bytes.windows(needle.len()).any(|w| w == needle),
            "{name} holds the address"
        );
    }
}
