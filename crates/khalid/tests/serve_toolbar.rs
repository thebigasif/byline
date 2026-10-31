//! The staff toolbar on a running `khalid serve` (plan section 4.6,
//! KH-EDT-6, brief G52): a signed-in owner sees a bar on a post with a
//! link to edit it, the lint result and its numbers, in an answer no cache
//! keeps; a reader gets the page untouched.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::{Value, json};
use tempfile::TempDir;

/// The demo post the test reads, and its address.
const PAGE: &str = "/blog/why-internal-links-matter/";

/// A running `khalid serve` and its address, stopped when dropped.
struct Server(Child, String);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A copy of the demo site, so the test can sign in to it.
#[cfg(test)]
fn demo_copy() -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    temp
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
    Server(child, host.trim_end_matches("/blog/").to_owned())
}

/// Sends `method` to `path` on `host` with `headers` and `body`; answers
/// the head and the body.
#[cfg(test)]
fn exchange(
    host: &str,
    (method, path): (&str, &str),
    headers: &str,
    body: &str,
) -> (String, String) {
    let mut stream = TcpStream::connect(host).unwrap();
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n{headers}\
         Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(head.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    (head.to_owned(), body.to_owned())
}

/// Makes the site's owner and answers the token of their sign-in link.
#[cfg(test)]
fn owner_token(site: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "--json", "staff", "owner"])
        .args(["--email", "asif@example.com", "--name", "Asif"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let made: Value = serde_json::from_slice(&output.stdout).unwrap();
    let link = made["link"].as_str().unwrap();
    link.split_once("#token=").unwrap().1.to_owned()
}

#[test]
fn a_signed_in_owner_sees_the_bar_and_a_reader_does_not() {
    let site = demo_copy();
    let token = owner_token(site.path());
    let server = start(site.path());
    let host = server.1.as_str();
    let redeem = json!({"token": token}).to_string();
    let path = "/api/admin/v1/session/link/redeem/";
    let (head, body) = exchange(host, ("POST", path), "", &redeem);
    assert!(head.starts_with("HTTP/1.1 200"), "{head}{body}");
    let cookie = head
        .lines()
        .find_map(|line| line.strip_prefix("set-cookie: "))
        .and_then(|value| value.split(';').next())
        .unwrap();
    let signed_in = format!("Cookie: {cookie}\r\nAccept-Encoding: br\r\n");

    let (head, page) = exchange(host, ("GET", PAGE), &signed_in, "");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(head.contains("cache-control: private, no-store"), "{head}");
    assert!(!head.contains("etag"), "{head}");
    let edit = "href=\"/blog/admin/posts/en.why-internal-links-matter\">Edit this post</a>";
    assert!(page.contains(edit), "{page}");
    assert!(
        page.contains("lint check") || page.contains("going live"),
        "{page}"
    );
    assert!(page.contains("in the last 30 days."), "{page}");
    assert!(page.contains("/blog/_toolbar.css"), "{page}");
    let (head, home) = exchange(host, ("GET", "/blog/"), &signed_in, "");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(
        home.contains("href=\"/blog/admin/\">Open the admin</a>"),
        "{home}"
    );

    let (head, css) = exchange(host, ("GET", "/blog/_toolbar.css"), "", "");
    assert!(head.contains("content-type: text/css"), "{head}");
    assert!(css.contains(".khalid-toolbar"), "{css}");
    let (head, page) = exchange(host, ("GET", PAGE), "", "");
    assert!(head.starts_with("HTTP/1.1 200"), "{head}");
    assert!(!page.contains("khalid-toolbar"), "a reader got the bar");
    let stale = "Cookie: __Host-khalid-session=nothing\r\n";
    let (_, page) = exchange(host, ("GET", PAGE), stale, "");
    assert!(
        !page.contains("khalid-toolbar"),
        "a dead session got the bar"
    );
}
