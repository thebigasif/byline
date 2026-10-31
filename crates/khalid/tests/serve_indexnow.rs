//! A live post whose file gets a new `updated:` date while `khalid serve`
//! runs is sent to `IndexNow`, as a post edited through the door is (task
//! C8, KH-SEO-11). `IndexNow` is a `wiremock` stand-in.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// The demo post the test updates, and its public address.
const POST: &str = "content/en/posts/2026-09-28-what-a-sitemap-does.md";
const ADDRESS: &str = "https://www.example.com/blog/what-a-sitemap-does/";

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

/// `rest` inside `dir`.
fn at(dir: &Path, rest: &str) -> PathBuf {
    let mut path = dir.to_path_buf();
    path.push(rest);
    path
}

/// A copy of the demo site sending `IndexNow` to `receiver`.
#[cfg(test)]
fn demo_site(receiver: &MockServer) -> TempDir {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    let port = receiver.address().port();
    let settings = at(temp.path(), "site.toml");
    let toml = format!(
        "{}\n[seo.indexnow]\nkey = \"3f9c0b5e2a8d4c71\"\n\
         endpoint = \"http://indexnow.test:{port}/indexnow\"\n",
        fs::read_to_string(&settings).unwrap()
    );
    fs::write(&settings, toml).unwrap();
    temp
}

/// Starts `khalid serve` on the site, reaching the stand-in on loopback.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap()])
        .args(["--test-hosts", "indexnow.test=127.0.0.1"])
        .args(["serve", "--listen", "127.0.0.1:0", "--json"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let json: Value = serde_json::from_str(&line).unwrap_or_else(|_| panic!("{line}"));
    let address = json["address"].as_str().unwrap().strip_prefix("http://");
    let host = address.unwrap().trim_end_matches("/blog/").to_owned();
    Server { child, host }
}

/// Waits until the server has read the site whole: the Content API answers
/// only once it has, and the last read is what a save is compared with.
#[cfg(test)]
fn wait_until_read(server: &Server, site: &Path) {
    let made = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "keys", "create", "--json"])
        .args(["--kind", "content", "--name", "Web page"])
        .output()
        .unwrap();
    let made: Value = serde_json::from_slice(&made.stdout).unwrap();
    let key = made["key"].as_str().unwrap();
    let mut stream = TcpStream::connect(&server.host).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .unwrap();
    let request = format!(
        "GET /api/content/v1/posts/?key={key} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        server.host
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
}

/// The `IndexNow` requests `receiver` got, once there is one, waiting up
/// to 30 seconds.
#[cfg(test)]
async fn told(receiver: &MockServer) -> Vec<Request> {
    for _ in 0..300 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let all = receiver.received_requests().await.unwrap();
        if !all.is_empty() {
            return all;
        }
    }
    Vec::new()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_updated_date_in_a_live_post_file_is_sent_to_indexnow() {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/indexnow"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&receiver)
        .await;
    let site = demo_site(&receiver);
    let server = start(site.path());
    wait_until_read(&server, site.path());
    assert!(
        receiver.received_requests().await.unwrap().is_empty(),
        "starting tells nothing"
    );

    let file = at(site.path(), POST);
    let text = fs::read_to_string(&file).unwrap();
    let updated = text.replacen(
        "date: 2026-09-28\n",
        "date: 2026-09-28\nupdated: 2026-10-30\n",
        1,
    );
    assert_ne!(text, updated, "the demo post's date line moved");
    fs::write(&file, updated).unwrap();

    let requests = told(&receiver).await;
    assert_eq!(requests.len(), 1, "IndexNow is told once");
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["urlList"], json!([ADDRESS]), "{body}");
}
