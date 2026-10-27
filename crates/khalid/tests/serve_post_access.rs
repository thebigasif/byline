//! A password post on a running `khalid serve`, end to end over HTTP with
//! no scripts: the page shows a form and never the body, a wrong password
//! shows the alert, a right one sets a cookie that opens the post, and the
//! full page has no other door (task 5.11, KH-EDT-3).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use khalid_api::post_password::NewPassword;
use serde_json::Value;

/// A password made up for this test.
const PASSWORD: &str = "generated-for-this-test";

/// The post, and a line of its body that only its full page shows.
const POST: &str = "/blog/why-internal-links-matter/";
const BODY: &str = "It sounds too simple to matter.";

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

/// A copy of the demo site whose first post has the test's password.
#[cfg(test)]
fn site_with_password() -> tempfile::TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    copy_dir(&demo, temp.path());
    let mut file = temp.path().to_path_buf();
    file.push("content/en/posts/2026-09-14-why-internal-links-matter.md");
    let text = fs::read_to_string(&file).unwrap();
    let hash = NewPassword::try_new(PASSWORD.to_owned())
        .unwrap()
        .hash()
        .unwrap();
    let line = format!("---\npassword_hash: \"{}\"\n", hash.as_str());
    fs::write(&file, text.replacen("---\n", &line, 1)).unwrap();
    temp
}

#[cfg(test)]
fn khalid(site: &Path, args: &[&str]) -> std::process::Output {
    let site = site.to_str().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site])
        .args(args)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    output
}

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
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line}"));
    let address = json["address"].as_str().unwrap();
    let host = address
        .strip_prefix("http://")
        .unwrap()
        .trim_end_matches("/blog/");
    Server {
        child,
        host: host.to_owned(),
    }
}

/// Sends a request and returns the status, the head and the body.
#[cfg(test)]
fn send(server: &Server, head: &str, body: &str) -> (u16, String, String) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let request = format!(
        "{head}\r\nHost: {host}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, head.to_owned(), body.to_owned())
}

/// Posts the password form with `password`.
#[cfg(test)]
fn unlock(server: &Server, password: &str) -> (u16, String, String) {
    let head = "POST /blog/_unlock/ HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded";
    let form = format!("path=%2Fblog%2Fwhy-internal-links-matter%2F&password={password}");
    send(server, head, &form)
}

/// The value of header `name` in `head`.
#[cfg(test)]
fn header<'h>(head: &'h str, name: &str) -> Option<&'h str> {
    head.lines()
        .filter_map(|line| line.split_once(": "))
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value)
}

#[test]
fn a_password_opens_the_post_with_no_script_and_nothing_else_does() {
    let site = site_with_password();
    let key = khalid(
        site.path(),
        &[
            "keys", "create", "--json", "--kind", "content", "--name", "Web",
        ],
    );
    let key: Value = serde_json::from_slice(&key.stdout).unwrap();
    let server = start(site.path());

    let (status, _, page) = send(&server, &format!("GET {POST} HTTP/1.1"), "");
    assert_eq!(status, 200, "{page}");
    assert!(
        page.contains("<form class=\"unlock\" method=\"post\""),
        "{page}"
    );
    assert!(!page.contains(BODY), "the locked page never shows the body");
    let hidden = format!("GET /_unlocked{POST} HTTP/1.1");
    let (status, _, page) = send(&server, &hidden, "");
    assert_eq!(status, 404, "the full page has no door of its own");
    assert!(!page.contains(BODY));

    let (status, head, page) = unlock(&server, "not-the-password");
    assert_eq!(status, 401, "{head}");
    assert!(
        page.contains("class=\"unlock-error\" role=\"alert\">"),
        "{page}"
    );
    assert!(header(&head, "set-cookie").is_none(), "{head}");
    assert!(header(&head, "content-security-policy").is_some(), "{head}");

    let (status, head, _) = unlock(&server, PASSWORD);
    assert_eq!(status, 303, "{head}");
    assert_eq!(header(&head, "location"), Some(POST));
    let cookie = header(&head, "set-cookie")
        .unwrap()
        .split(';')
        .next()
        .unwrap();
    let opened = format!("GET {POST} HTTP/1.1\r\nCookie: {cookie}");
    let (status, head, page) = send(&server, &opened, "");
    assert_eq!(status, 200, "{head}");
    assert!(page.contains(BODY), "the cookie opens the post");
    assert_eq!(header(&head, "cache-control"), Some("private, no-store"));
    let elsewhere = format!("GET /blog/what-a-sitemap-does/ HTTP/1.1\r\nCookie: {cookie}");
    assert_eq!(send(&server, &elsewhere, "").0, 200);

    let key = key["key"].as_str().unwrap();
    let list = format!("GET /api/content/v1/posts/?key={key}&filter=language:en HTTP/1.1");
    let (status, _, body) = send(&server, &list, "");
    assert_eq!(status, 200, "{body}");
    assert!(!body.contains(BODY), "the Content API sends no locked body");
}
