//! Each webhook event fires through a running `khalid serve`, from the code
//! that causes it, to a stand-in receiver on loopback, signed with the
//! webhook's secret (task 1.19, KH-API-3, audit brief G25).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use khalid_core::{AbsoluteUrl, Secret, Slug, SystemClock};
use khalid_db::{Database, WebhookTable};
use khalid_webhooks::{Event, SIGNATURE_HEADER, SigningKey, Verifier};
use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// The stand-in host for the webhook receiver, on loopback.
const HOSTS: &str = "hooks.test=127.0.0.1";

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

/// A receiver that takes every delivery.
#[cfg(test)]
async fn receiver() -> MockServer {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&receiver)
        .await;
    receiver
}

/// A copy of the demo site with a webhook for `event` sending to
/// `receiver` at `/<event>`. Returns the site and the webhook's secret.
#[cfg(test)]
async fn demo_site(receiver: &MockServer, event: Event) -> (TempDir, Secret<String>) {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    let mut file = temp.path().to_path_buf();
    file.push("data");
    fs::create_dir_all(&file).unwrap();
    file.push("site.db");
    let db = Database::open(&file).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let port = receiver.address().port();
    let url = AbsoluteUrl::parse(&format!("http://hooks.test:{port}/{event}")).unwrap();
    let table = WebhookTable::new(db.clone(), site, SystemClock);
    let (_, secret) = table.create(&url, event).await.unwrap();
    db.close().await;
    (temp, secret)
}

/// Starts `khalid serve`, reaching the stand-in host on loopback.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap()])
        .args(["--test-hosts", HOSTS])
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

/// Sends `method` to `path` with `headers` and `body`; returns the status
/// and the body.
#[cfg(test)]
fn send(
    server: &Server,
    (method, path): (&str, &str),
    headers: &[&str],
    body: &str,
) -> (u16, String) {
    let mut stream = TcpStream::connect(&server.host).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n",
        server.host,
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
    (status, body.to_owned())
}

/// A key that may write and publish posts, made by `khalid keys create`.
#[cfg(test)]
fn writer_key(site: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "keys", "create", "--json"])
        .args(["--name", "SerpDad", "--scope", "posts:write"])
        .args(["--scope", "posts:publish"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["key"].as_str().unwrap().to_owned()
}

/// Sends `body` as JSON to the Admin API at `path` with `key`; returns the
/// status and the answer as JSON.
#[cfg(test)]
fn api(server: &Server, key: &str, (method, path): (&str, &str), body: &Value) -> (u16, Value) {
    let auth = format!("Authorization: Khalid {key}");
    let headers = [auth.as_str(), "Content-Type: application/json"];
    let (status, answer) = send(server, (method, path), &headers, &body.to_string());
    (status, serde_json::from_str(&answer).unwrap_or(Value::Null))
}

/// The deliveries of `event` that reached `receiver`, once there is one,
/// waiting up to 20 seconds.
#[cfg(test)]
async fn delivered(receiver: &MockServer, event: Event) -> Vec<Request> {
    let route = format!("/{event}");
    for _ in 0..200 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let all = receiver.received_requests().await.unwrap();
        let found: Vec<_> = all.into_iter().filter(|r| r.url.path() == route).collect();
        if !found.is_empty() {
            return found;
        }
    }
    Vec::new()
}

/// The one delivery of `event`, checked against `secret`, as JSON.
#[cfg(test)]
async fn the_delivery(receiver: &MockServer, event: Event, secret: &Secret<String>) -> Value {
    let hooks = delivered(receiver, event).await;
    assert_eq!(hooks.len(), 1, "{event} fires once");
    let hook = &hooks[0];
    assert_eq!(hook.headers["khalid-event"], event.as_str());
    let signature = hook.headers[SIGNATURE_HEADER].to_str().unwrap();
    let verifier = Verifier::new(SigningKey::try_new(secret).unwrap());
    verifier
        .verify(signature, &hook.body, &SystemClock)
        .unwrap();
    let body: Value = serde_json::from_slice(&hook.body).unwrap();
    assert_eq!(body["event"], event.as_str(), "{body}");
    body
}

#[tokio::test(flavor = "multi_thread")]
async fn a_reader_comment_fires_comment_added_without_their_email() {
    let receiver = receiver().await;
    let (site, secret) = demo_site(&receiver, Event::CommentAdded).await;
    let server = start(site.path());
    for _ in 0..200 {
        let (_, page) = send(&server, ("GET", "/blog/what-a-sitemap-does/"), &[], "");
        if page.contains("<form class=\"comment-form\"") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let form = "Content-Type: application/x-www-form-urlencoded";
    let fields = "page=%2Fblog%2Fwhat-a-sitemap-does%2F&name=Lena\
        &email=lena%40example.com&comment=Very+clear%2C+thanks.&website=";
    let (status, page) = send(&server, ("POST", "/blog/comments/"), &[form], fields);
    assert_eq!(status, 303, "{page}");

    let body = the_delivery(&receiver, Event::CommentAdded, &secret).await;
    assert_eq!(body["comment"]["text"], "Very clear, thanks.", "{body}");
    assert_eq!(body["comment"]["status"], "pending", "{body}");
    assert!(
        !body.to_string().contains("lena@example.com"),
        "the reader's email stays private: {body}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_change_to_a_live_post_fires_post_published_edited() {
    let receiver = receiver().await;
    let (site, secret) = demo_site(&receiver, Event::PostPublishedEdited).await;
    let key = writer_key(site.path());
    let server = start(site.path());

    let route = "/api/admin/v1/posts/en.what-a-sitemap-does/";
    let (status, live) = api(&server, &key, ("GET", route), &Value::Null);
    assert_eq!(status, 200, "{live}");
    let retitle = json!({"posts": [{"title": "What a sitemap does for you",
        "updated_at": live["posts"][0]["updated_at"]}]});
    let (status, edited) = api(&server, &key, ("PUT", route), &retitle);
    assert_eq!(status, 200, "{edited}");

    let body = the_delivery(&receiver, Event::PostPublishedEdited, &secret).await;
    assert_eq!(body["post"]["id"], "en.what-a-sitemap-does", "{body}");
    assert_eq!(body["post"]["title"], "What a sitemap does for you");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_post_the_linter_refuses_fires_lint_blocked_with_its_findings() {
    let receiver = receiver().await;
    let (site, secret) = demo_site(&receiver, Event::LintBlocked).await;
    let key = writer_key(site.path());
    let server = start(site.path());

    let new = json!({"posts": [{"title": "A weak post", "markdown": "Short."}]});
    let (status, made) = api(&server, &key, ("POST", "/api/admin/v1/posts/"), &new);
    assert_eq!(status, 201, "{made}");
    let publish = json!({"posts": [{"status": "published",
        "updated_at": made["posts"][0]["updated_at"]}]});
    let route = "/api/admin/v1/posts/en.a-weak-post/";
    let (status, refused) = api(&server, &key, ("PUT", route), &publish);
    assert_eq!(status, 422, "{refused}");

    let body = the_delivery(&receiver, Event::LintBlocked, &secret).await;
    assert_eq!(body["post"]["id"], "en.a-weak-post", "{body}");
    assert_eq!(body["post"]["status"], "draft", "{body}");
    assert_eq!(
        body["lint"], refused["lint"],
        "the same findings the API gave"
    );
}
