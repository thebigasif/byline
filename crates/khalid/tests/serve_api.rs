//! The APIs on a running `khalid serve`: the Content API answers from the
//! site's own index with a key made by `khalid keys create`, and shows a
//! saved edit without a restart (KH-API-1, KH-API-2).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use tempfile::TempDir;

/// The pause between looks for an edit.
const PAUSE: Duration = Duration::from_millis(100);

/// How many looks, 20 seconds in all, before the test fails.
const TRIES: u32 = 200;

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

/// A copy of the demo site, so a test can change it.
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

/// Runs `khalid --site <site> keys create --json` with `args` and returns
/// the key it showed.
#[cfg(test)]
fn new_key(site: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "keys", "create", "--json"])
        .args(args)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["key"].as_str().unwrap().to_owned()
}

/// Starts `khalid serve` on `site` on a free port and reads the address it
/// prints.
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

/// Sends `method` to `path` with `headers` and a JSON `body`, if any, and
/// returns the status and the body.
#[cfg(test)]
fn request(
    server: &Server,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&Value>,
) -> (u16, String) {
    let host = &server.host;
    let mut stream = TcpStream::connect(host).unwrap();
    let body = body.map(Value::to_string).unwrap_or_default();
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    if !body.is_empty() {
        head.push_str("Content-Type: application/json\r\n");
    }
    for (name, value) in headers {
        head = format!("{head}{name}: {value}\r\n");
    }
    stream
        .write_all(format!("{head}\r\n{body}").as_bytes())
        .unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    (
        head.split(' ').nth(1).unwrap().parse().unwrap(),
        body.to_owned(),
    )
}

/// The English posts, newest first, as the Content API lists them.
#[cfg(test)]
fn english_posts(server: &Server, key: &str) -> Value {
    let path = format!("/api/content/v1/posts/?key={key}&filter=language:en");
    let (status, body) = request(server, "GET", &path, &[], None);
    assert_eq!(status, 200, "{body}");
    serde_json::from_str(&body).unwrap()
}

#[test]
fn the_content_api_lists_the_demo_posts_and_shows_an_edit() {
    let site = demo_copy();
    let key = new_key(site.path(), &["--kind", "content", "--name", "Web page"]);
    let server = start(site.path());

    let listed = english_posts(&server, &key);
    let posts = listed["posts"].as_array().unwrap();
    let slugs: Vec<&str> = posts.iter().filter_map(|p| p["slug"].as_str()).collect();
    assert_eq!(
        slugs,
        [
            "planning-a-topic-cluster",
            "fixing-slow-pages",
            "what-a-sitemap-does",
            "how-to-write-a-title-tag",
            "why-internal-links-matter",
        ]
    );
    assert_eq!(listed["meta"]["pagination"]["total"], 5, "{listed}");

    let (status, _) = request(&server, "GET", "/api/content/v1/posts/", &[], None);
    assert_eq!(status, 401, "no key, no posts");

    let mut file = site.path().to_path_buf();
    file.push("content/en/posts/2026-09-28-what-a-sitemap-does.md");
    let post = fs::read_to_string(&file).unwrap();
    fs::write(
        &file,
        post.replace("title: What a sitemap does", "title: What a sitemap is for"),
    )
    .unwrap();
    let shown = (0..TRIES).any(|_| {
        thread::sleep(PAUSE);
        english_posts(&server, &key)
            .to_string()
            .contains("What a sitemap is for")
    });
    assert!(shown, "the edit never reached the Content API");
}
