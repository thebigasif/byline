//! Comments on a running `khalid serve`, end to end and with no script: a
//! reader posts the plain form under a post, the comment waits, a bot's
//! comment is kept nowhere, a mistake comes back as a page, a flood is
//! slowed down, and once a moderator approves it through the Admin API the
//! post page and its schema show it (task 5.4, KH-WEB-2, KH-SEO-3,
//! BL-SEC-5).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;

/// The pause between looks for a change, and how many looks, 20 seconds
/// in all, before the test fails.
const PAUSE: Duration = Duration::from_millis(100);
const TRIES: u32 = 200;

const POST: &str = "/blog/what-a-sitemap-does/";

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

/// A copy of the demo site, with a key that may moderate comments, served.
#[cfg(test)]
fn served_demo() -> (TempDir, Server, String) {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let site = tempfile::tempdir().unwrap();
    copy_dir(&demo, site.path());
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.path().to_str().unwrap(), "keys", "create"])
        .args(["--json", "--name", "Moderator", "--scope", "posts:write"])
        .output()
        .unwrap();
    let key: Value = serde_json::from_slice(&output.stdout).unwrap();
    let key = key["key"].as_str().unwrap().to_owned();
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.path().to_str().unwrap(), "serve", "--listen"])
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
    let host = address.strip_prefix("http://").unwrap();
    let host = host.trim_end_matches("/blog/").to_owned();
    (site, Server { child, host }, key)
}

/// Sends one request and returns the status, the headers and the body.
#[cfg(test)]
fn send(
    server: &Server,
    (method, path): (&str, &str),
    headers: &[&str],
    body: &str,
) -> (u16, String, String) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for header in headers {
        head = format!("{head}{header}\r\n");
    }
    stream
        .write_all(format!("{head}\r\n{body}").as_bytes())
        .unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, head.to_ascii_lowercase(), body.to_owned())
}

/// Posts the comment form under the post, as a browser does with scripts off.
#[cfg(test)]
fn comment(server: &Server, fields: &str) -> (u16, String, String) {
    let form = "Content-Type: application/x-www-form-urlencoded";
    let body = format!("page=%2Fblog%2Fwhat-a-sitemap-does%2F&{fields}");
    send(server, ("POST", "/blog/comments/"), &[form], &body)
}

#[cfg(test)]
fn queue(server: &Server, key: &str) -> Vec<Value> {
    let auth = format!("Authorization: Khalid {key}");
    let path = "/api/admin/v1/comments/?status=pending";
    let (status, _, body) = send(server, ("GET", path), &[&auth], "");
    assert_eq!(status, 200, "{body}");
    let json: Value = serde_json::from_str(&body).unwrap();
    json["comments"].as_array().unwrap().clone()
}

/// The post page once it carries `text`, or the last one seen.
#[cfg(test)]
fn post_page_with(server: &Server, text: &str) -> String {
    let mut page = String::new();
    for _ in 0..TRIES {
        page = send(server, ("GET", POST), &[], "").2;
        if page.contains(text) {
            break;
        }
        thread::sleep(PAUSE);
    }
    page
}

#[test]
fn a_comment_sent_without_script_waits_then_shows_once_approved() {
    let (_site, server, key) = served_demo();
    let page = post_page_with(&server, "<form class=\"comment-form\"");
    assert!(page.contains("action=\"/blog/comments/\""), "{page}");
    assert!(page.contains("No comments yet."));
    let sent = comment(
        &server,
        "name=Lena&email=&comment=Very+clear%2C+thanks.&website=",
    );
    assert_eq!(sent.0, 303);
    let waiting = queue(&server, &key);
    assert_eq!(waiting.len(), 1, "{waiting:?}");
    assert_eq!(waiting[0]["text"], "Very clear, thanks.");
    assert!(
        !send(&server, ("GET", POST), &[], "")
            .2
            .contains("Very clear")
    );

    let id = waiting[0]["id"].as_i64().unwrap();
    let auth = format!("Authorization: Khalid {key}");
    let approve = format!("/api/admin/v1/comments/{id}/approve/");
    assert_eq!(send(&server, ("POST", &approve), &[&auth], "").0, 200);
    let page = post_page_with(&server, "Very clear, thanks.");
    assert!(page.contains(&format!("id=\"comment-{id}\"")), "{page}");
    assert!(
        page.contains("\"@type\":\"Comment\""),
        "the schema lists it"
    );

    let delete = format!("/api/admin/v1/comments/{id}/");
    assert_eq!(send(&server, ("DELETE", &delete), &[&auth], "").0, 204);
    let page = post_page_with(&server, "No comments yet.");
    assert!(
        !page.contains("Very clear"),
        "a deleted comment leaves the page"
    );
}

#[test]
fn a_mistake_comes_back_as_a_page_and_a_flood_is_slowed() {
    let (_site, server, key) = served_demo();
    post_page_with(&server, "<form class=\"comment-form\"");
    let (status, _, page) = comment(&server, "name=&comment=Hello");
    assert_eq!(status, 422);
    assert!(page.contains("Please give your name."), "{page}");
    assert!(page.contains(">Hello</textarea>"));
    let bait = comment(
        &server,
        "name=Bot&comment=Buy+now&website=https%3A%2F%2Fspam.example",
    );
    assert_eq!(bait.0, 303, "a bot is answered like a reader");
    for _ in 0..3 {
        let (status, head, _) = comment(&server, "name=Ben&comment=Again");
        assert_eq!(status, 303);
        let back = "location: /blog/what-a-sitemap-does/#comment-sent";
        assert!(head.contains(back), "{head}");
    }
    let (status, head, page) = comment(&server, "name=Ben&comment=Again");
    assert_eq!(status, 429);
    assert!(head.contains("retry-after: 600"), "{head}");
    assert!(page.contains("try again in 10 minutes"));
    assert_eq!(queue(&server, &key).len(), 3, "the bait was kept nowhere");
}
