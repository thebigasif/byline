//! Stored redirects through `khalid serve` (KH-SEO-8): each status and a
//! regex rule answer from the moment the Admin API stores them, chains
//! are one step, and a page always answers before a stored rule.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::{Value, json};
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

/// A copy of the demo site without any `data/` a local build left in it.
#[cfg(test)]
fn demo_copy() -> TempDir {
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
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site");
    let temp = tempfile::tempdir().unwrap();
    for name in ["site.toml", "categories.toml", "content"] {
        let (mut from, mut into) = (demo.clone(), temp.path().to_path_buf());
        from.push(name);
        into.push(name);
        if from.is_dir() {
            copy_dir(&from, &into);
        } else {
            fs::copy(&from, &into).unwrap();
        }
    }
    temp
}

/// A key with `settings:write` for `site`, made with `khalid keys create`.
#[cfg(test)]
fn settings_key(site: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "keys", "create", "--json"])
        .args(["--name", "Redirects", "--scope", "settings:write"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["key"].as_str().unwrap().to_owned()
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

/// Sends `method` to `path`, with `key` and a JSON `body` if given, and
/// returns the status, the head lines in lower case, and the body.
#[cfg(test)]
fn send(
    server: &Server,
    method: &str,
    path: &str,
    key: Option<&str>,
    body: Option<&Value>,
) -> (u16, String, String) {
    let mut stream = TcpStream::connect(&server.host).unwrap();
    let body = body.map(Value::to_string).unwrap_or_default();
    let (host, length) = (&server.host, body.len());
    let authorization = key.map_or(String::new(), |key| {
        format!("Authorization: Khalid {key}\r\n")
    });
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\
         Content-Length: {length}\r\nContent-Type: application/json\r\n{authorization}"
    );
    stream
        .write_all(format!("{head}\r\n{body}").as_bytes())
        .unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, head.to_lowercase(), body.to_owned())
}

/// Stores `redirect` through the Admin API.
#[cfg(test)]
fn store(server: &Server, key: &str, redirect: &Value) {
    let body = json!({"redirects": [redirect]});
    let (status, _, answer) = send(
        server,
        "POST",
        "/api/admin/v1/redirects/",
        Some(key),
        Some(&body),
    );
    assert_eq!(status, 201, "{answer}");
}

#[test]
fn stored_redirects_answer_through_khalid_serve_in_one_step() {
    let site = demo_copy();
    let key = settings_key(site.path());
    let server = start(site.path());
    let (status, ..) = send(&server, "GET", "/blog/old-links/", None, None);
    assert_eq!(status, 404);

    store(
        &server,
        &key,
        &json!({"from": "/blog/old-links/", "to": "/blog/why-internal-links-matter/", "status": 301}),
    );
    let (status, head, _) = send(
        &server,
        "GET",
        "/blog/old-links/?ref=newsletter",
        None,
        None,
    );
    assert_eq!(status, 301, "{head}");
    assert!(
        head.contains("\r\nlocation: /blog/why-internal-links-matter/?ref=newsletter"),
        "{head}"
    );
    assert!(head.contains("\r\ncontent-security-policy: "), "{head}");

    store(
        &server,
        &key,
        &json!({"from": "/blog/\\d{4}/(.+)", "to": "/blog/$1", "status": 302, "match": "regex"}),
    );
    let (status, head, _) = send(&server, "GET", "/blog/2019/old-links/", None, None);
    assert_eq!(status, 302, "{head}");
    assert!(
        head.contains("\r\nlocation: /blog/why-internal-links-matter/\r\n"),
        "{head}"
    );

    store(
        &server,
        &key,
        &json!({"from": "/blog/retired-tool/", "status": 410}),
    );
    let (status, head, body) = send(&server, "GET", "/blog/retired-tool/", None, None);
    assert_eq!(status, 410, "{head}");
    assert!(body.contains("<html"), "{body}");

    store(
        &server,
        &key,
        &json!({"from": "/blog/why-internal-links-matter/", "to": "/blog/", "status": 301}),
    );
    let (status, ..) = send(
        &server,
        "GET",
        "/blog/why-internal-links-matter/",
        None,
        None,
    );
    assert_eq!(status, 200, "a page answers before a stored redirect");
}

/// The 404 log as the Admin API lists it, once it holds `path`.
#[cfg(test)]
fn logged(server: &Server, key: &str, path: &str) -> Value {
    for _ in 0..100 {
        let (_, _, body) = send(server, "GET", "/api/admin/v1/not-found/", Some(key), None);
        let json: Value = serde_json::from_str(&body).unwrap();
        if json["misses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["path"] == path)
        {
            return json;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("{path} never reached the 404 log");
}

#[test]
fn a_404_is_logged_and_redirected_in_one_step_for_good() {
    let site = demo_copy();
    let key = settings_key(site.path());
    let server = start(site.path());
    for path in [
        "/blog/internal-links/",
        "/api/content/v1/nothing/",
        "/elsewhere/",
    ] {
        let (status, ..) = send(&server, "GET", path, None, None);
        assert_eq!(status, 404, "{path}");
    }
    let log = logged(&server, &key, "/blog/internal-links/");
    assert_eq!(
        log["misses"].as_array().unwrap().len(),
        1,
        "only the blog's own 404: {log}"
    );
    assert_eq!(
        log["misses"][0]["nearest"],
        "/blog/why-internal-links-matter/"
    );

    let body = json!({"path": "/blog/internal-links/"});
    let route = "/api/admin/v1/not-found/redirect/";
    let (status, _, answer) = send(&server, "POST", route, Some(&key), Some(&body));
    assert_eq!(status, 201, "{answer}");
    drop(server);

    let server = start(site.path());
    let (status, head, _) = send(&server, "GET", "/blog/internal-links/", None, None);
    assert_eq!(status, 301, "{head}");
    assert!(
        head.contains("\r\nlocation: /blog/why-internal-links-matter/\r\n"),
        "{head}"
    );
}
