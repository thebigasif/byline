//! Publishing on a running `khalid serve` through the Admin API fires the
//! signed `post.published` webhook to the site's webhook list and sends
//! the post's address to `IndexNow`, once each (KH-API-3, KH-SEO-11, CTO
//! answers 2026-10-27 section 4).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use chrono::TimeDelta;
use khalid_core::{AbsoluteUrl, Clock, Secret, Slug, SystemClock};
use khalid_db::{Database, WebhookTable};
use khalid_webhooks::{Event, SIGNATURE_HEADER, SigningKey, Verifier};
use serde_json::{Value, json};
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// The stand-in hosts for the webhook receiver and `IndexNow`, on loopback.
const HOSTS: &str = "hooks.test=127.0.0.1,indexnow.test=127.0.0.1";

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

/// A copy of the demo site with `IndexNow` sent to `receiver`, and a
/// `post.published` webhook there. Returns the site and the secret.
#[cfg(test)]
async fn demo_site(receiver: &MockServer) -> (TempDir, Secret<String>) {
    let mut demo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    demo.push("../../fixtures/demo-site/.");
    let temp = tempfile::tempdir().unwrap();
    let copied = Command::new("cp")
        .arg("-R")
        .args([&demo, temp.path()])
        .status();
    assert!(copied.unwrap().success());
    let port = receiver.address().port();
    let mut settings = temp.path().to_path_buf();
    settings.push("site.toml");
    let toml = format!(
        "{}\n[seo.indexnow]\nkey = \"3f9c0b5e2a8d4c71\"\n\
         endpoint = \"http://indexnow.test:{port}/indexnow\"\n",
        fs::read_to_string(&settings).unwrap()
    );
    fs::write(&settings, toml).unwrap();
    let mut file = temp.path().to_path_buf();
    file.push("data");
    fs::create_dir_all(&file).unwrap();
    file.push("site.db");
    let db = Database::open(&file).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let url = AbsoluteUrl::parse(&format!("http://hooks.test:{port}/khalid")).unwrap();
    let table = WebhookTable::new(db.clone(), site, SystemClock);
    let (_, secret) = table.create(&url, Event::PostPublished).await.unwrap();
    db.close().await;
    (temp, secret)
}

/// A key that may write posts, made by `khalid keys create`.
#[cfg(test)]
fn writer_key(site: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap(), "keys", "create", "--json"])
        .args(["--name", "SerpDad", "--scope", "posts:write"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    json["key"].as_str().unwrap().to_owned()
}

/// Starts `khalid serve`, reaching the stand-in hosts on loopback.
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

/// Sends `method` to `path` with the key and a JSON `body`; returns the
/// status and the body as JSON.
#[cfg(test)]
fn send(server: &Server, key: &str, method: &str, path: &str, body: &Value) -> (u16, Value) {
    let mut stream = TcpStream::connect(&server.host).unwrap();
    let body = body.to_string();
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\
         Authorization: Khalid {key}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\n\r\n",
        server.host,
        body.len()
    );
    stream
        .write_all(format!("{head}{body}").as_bytes())
        .unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

/// The requests `receiver` got at `route`, once there is one, waiting up
/// to 20 seconds.
#[cfg(test)]
async fn received(receiver: &MockServer, route: &str) -> Vec<Request> {
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

#[tokio::test(flavor = "multi_thread")]
async fn a_post_published_through_the_api_fires_the_webhook_and_indexnow() {
    let receiver = MockServer::start().await;
    for route in ["/khalid", "/indexnow"] {
        Mock::given(method("POST"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200))
            .mount(&receiver)
            .await;
    }
    let (site, secret) = demo_site(&receiver).await;
    let key = writer_key(site.path());
    let server = start(site.path());

    let new = json!({"posts": [{"title": "Choosing anchor text", "author": "asif",
        "markdown": "Anchor text is the visible words of a link.\n\n\
        Read [why internal links matter](/blog/why-internal-links-matter/) first."}]});
    let (status, made) = send(&server, &key, "POST", "/api/admin/v1/posts/", &new);
    assert_eq!(status, 201, "{made}");
    let publish = json!({"posts": [{"status": "published",
        "updated_at": made["posts"][0]["updated_at"]}]});
    let route = "/api/admin/v1/posts/en.choosing-anchor-text/";
    let (status, live) = send(&server, &key, "PUT", route, &publish);
    assert_eq!(status, 200, "{live}");

    let hooks = received(&receiver, "/khalid").await;
    assert_eq!(hooks.len(), 1, "post.published fires once");
    let hook = &hooks[0];
    assert_eq!(hook.headers["khalid-event"], "post.published");
    let signature = hook.headers[SIGNATURE_HEADER].to_str().unwrap();
    let verifier = Verifier::new(SigningKey::try_new(&secret).unwrap());
    verifier
        .verify(signature, &hook.body, &SystemClock)
        .unwrap();
    let body: Value = serde_json::from_slice(&hook.body).unwrap();
    assert_eq!(body["event"], "post.published");
    assert_eq!(body["post"]["id"], "en.choosing-anchor-text", "{body}");
    assert_eq!(body["post"]["status"], "published", "{body}");

    let searches = received(&receiver, "/indexnow").await;
    assert_eq!(searches.len(), 1, "IndexNow is told once");
    let search: Value = serde_json::from_slice(&searches[0].body).unwrap();
    assert_eq!(
        search["urlList"],
        json!(["https://www.example.com/blog/choosing-anchor-text/"])
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_post_scheduled_through_the_api_goes_live_at_its_time_and_is_announced() {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&receiver)
        .await;
    let (site, _) = demo_site(&receiver).await;
    let key = writer_key(site.path());
    let server = start(site.path());
    let new = json!({"posts": [{"title": "Choosing anchor text", "author": "asif",
        "markdown": "Anchor text is the visible words of a link.\n\n\
        Read [why internal links matter](/blog/why-internal-links-matter/) first."}]});
    let (_, made) = send(&server, &key, "POST", "/api/admin/v1/posts/", &new);

    // Far enough ahead that the edit lands first on a loaded machine, and
    // well inside the 20 seconds `received` waits.
    let soon = SystemClock.now() + TimeDelta::seconds(8);
    let schedule = json!({"posts": [{"status": "scheduled", "published_at": soon,
        "updated_at": made["posts"][0]["updated_at"]}]});
    let route = "/api/admin/v1/posts/en.choosing-anchor-text/";
    let (status, scheduled) = send(&server, &key, "PUT", route, &schedule);
    assert_eq!(status, 200, "{scheduled}");
    let page = "/blog/choosing-anchor-text/";

    let hooks = received(&receiver, "/khalid").await;
    assert_eq!(hooks.len(), 1, "post.published fires once, at its time");
    let body: Value = serde_json::from_slice(&hooks[0].body).unwrap();
    assert_eq!(body["post"]["updated_by"], "scheduler", "{body}");
    let searches = received(&receiver, "/indexnow").await;
    assert_eq!(searches.len(), 1, "IndexNow is told once, at its time");
    let search: Value = serde_json::from_slice(&searches[0].body).unwrap();
    let address = "https://www.example.com/blog/choosing-anchor-text/";
    assert_eq!(search["urlList"], json!([address]));
    let shown = (0..200).any(|_| {
        std::thread::sleep(Duration::from_millis(100));
        send(&server, &key, "GET", page, &Value::Null).0 == 200
    });
    assert!(shown, "the scheduled post was never served");
}

/// Gets `path` with no key; returns the status, the head in lowercase and
/// the body.
#[cfg(test)]
fn page(server: &Server, path: &str) -> (u16, String, String) {
    let mut stream = TcpStream::connect(&server.host).unwrap();
    let host = &server.host;
    let head = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(head.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    (status, head.to_lowercase(), body.to_owned())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_preview_link_shows_the_draft_with_noindex_and_nothing_else() {
    let receiver = MockServer::start().await;
    let (site, _) = demo_site(&receiver).await;
    let key = writer_key(site.path());
    let server = start(site.path());
    let new = json!({"posts": [{"title": "Choosing anchor text", "author": "asif",
        "markdown": "Anchor text is the visible words of a link."}]});
    send(&server, &key, "POST", "/api/admin/v1/posts/", &new);
    let route = "/api/admin/v1/posts/en.choosing-anchor-text/preview-link/";
    let (status, links) = send(&server, &key, "POST", route, &Value::Null);
    assert_eq!(status, 201, "{links}");
    let url = links["preview_links"][0]["url"].as_str().unwrap();
    let path = url.strip_prefix("https://www.example.com").unwrap();

    let (status, head, body) = page(&server, path);
    assert_eq!(status, 200, "{body}");
    assert!(
        body.contains("Anchor text is the visible words of a link."),
        "{body}"
    );
    assert!(body.contains("<meta name=\"robots\" content=\"noindex, nofollow\">"));
    assert!(head.contains("x-robots-tag: noindex, nofollow"), "{head}");
    assert!(head.contains("content-security-policy: "), "{head}");
    assert!(head.contains("cache-control: no-store"), "{head}");

    let changed = path.replace("sig=", "sig=00");
    assert_eq!(
        page(&server, &changed).0,
        404,
        "a changed link opens nothing"
    );
    let public = page(&server, "/blog/choosing-anchor-text/").0;
    assert_eq!(public, 404, "the draft is still not public");
}
