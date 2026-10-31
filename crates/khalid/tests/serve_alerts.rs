//! Alerts through a running `khalid serve` (KH-GRO-6, G44): stale posts
//! and a burst of 404s reach the `alert.raised` webhook, signed, and a
//! problem already raised is not sent again.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use khalid_core::{AbsoluteUrl, Secret, Slug, SystemClock};
use khalid_db::{Database, WebhookTable};
use khalid_webhooks::{Event, SIGNATURE_HEADER, SigningKey, Verifier};
use serde_json::Value;
use tempfile::TempDir;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

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

/// A copy of the demo site where a post is stale after a day and three
/// new 404s are a spike, with an `alert.raised` webhook to `receiver`.
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
    let mut settings = temp.path().to_path_buf();
    settings.push("site.toml");
    let mut file = OpenOptions::new().append(true).open(&settings).unwrap();
    writeln!(
        file,
        "\n[alerts]\nnot_found_spike = 3\nstale_after_days = 1"
    )
    .unwrap();
    let mut path = temp.path().to_path_buf();
    path.push("data");
    fs::create_dir_all(&path).unwrap();
    path.push("site.db");
    let db = Database::open(&path).await.unwrap();
    db.migrate().await.unwrap();
    let site = db.site(&Slug::try_new("main").unwrap()).await.unwrap();
    let port = receiver.address().port();
    let url = AbsoluteUrl::parse(&format!("http://hooks.test:{port}/alerts")).unwrap();
    let table = WebhookTable::new(db.clone(), site, SystemClock);
    let (_, secret) = table.create(&url, Event::AlertRaised).await.unwrap();
    db.close().await;
    (temp, secret)
}

/// Starts `khalid serve` with a check every second, reaching the receiver
/// on loopback.
#[cfg(test)]
fn start(site: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_khalid"))
        .args(["--site", site.to_str().unwrap()])
        .args(["--test-hosts", "hooks.test=127.0.0.1"])
        .args([
            "serve",
            "--listen",
            "127.0.0.1:0",
            "--json",
            "--alert-every",
            "1",
        ])
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

/// Asks for `path` and returns the status.
#[cfg(test)]
fn get(server: &Server, path: &str) -> u16 {
    let mut stream = TcpStream::connect(&server.host).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    let host = &server.host;
    let head = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(head.as_bytes()).unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();
    raw.split(' ').nth(1).unwrap().parse().unwrap()
}

/// Every alert delivered so far, checked against `secret`, as JSON.
#[cfg(test)]
async fn alerts(receiver: &MockServer, secret: &Secret<String>) -> Vec<Value> {
    let verifier = Verifier::new(SigningKey::try_new(secret).unwrap());
    let hooks = receiver.received_requests().await.unwrap();
    hooks
        .iter()
        .map(|hook| {
            let signature = hook.headers[SIGNATURE_HEADER].to_str().unwrap();
            verifier
                .verify(signature, &hook.body, &SystemClock)
                .unwrap();
            serde_json::from_slice(&hook.body).unwrap()
        })
        .collect()
}

/// The alerts of `kind` once there is one, waiting up to 30 seconds.
#[cfg(test)]
async fn of_kind(receiver: &MockServer, secret: &Secret<String>, kind: &str) -> Vec<Value> {
    for _ in 0..300 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let found: Vec<Value> = alerts(receiver, secret)
            .await
            .into_iter()
            .filter(|body| body["alert"]["kind"] == kind)
            .collect();
        if !found.is_empty() {
            return found;
        }
    }
    Vec::new()
}

#[tokio::test(flavor = "multi_thread")]
async fn stale_posts_and_a_burst_of_404s_reach_the_alert_webhook_once_each() {
    let receiver = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&receiver)
        .await;
    let (site, secret) = demo_site(&receiver).await;
    let server = start(site.path());

    let stale = of_kind(&receiver, &secret, "stale-post").await;
    assert_eq!(stale.len(), 1, "one alert for every stale post");
    let alert = &stale[0]["alert"];
    assert_eq!(stale[0]["event"], "alert.raised");
    assert_eq!(
        alert["what"],
        "Some live posts have not changed for a long time."
    );
    let findings = alert["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["key"] == "stale-post:en.why-internal-links-matter"),
        "{alert}"
    );

    for missing in ["/blog/gone/", "/blog/gone/", "/blog/moved/", "/blog/gone/"] {
        assert_eq!(get(&server, missing), 404);
    }
    let spikes = of_kind(&receiver, &secret, "not-found-spike").await;
    let lines: Vec<&str> = spikes[0]["alert"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["line"].as_str().unwrap())
        .collect();
    assert_eq!(
        lines,
        ["/blog/gone/: 3 more 404s", "/blog/moved/: 1 more 404s"]
    );

    // Later rounds find the same stale posts, and send nothing new.
    tokio::time::sleep(Duration::from_secs(3)).await;
    let all = alerts(&receiver, &secret).await;
    let kinds: Vec<&str> = all
        .iter()
        .map(|b| b["alert"]["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds.iter().filter(|kind| **kind == "stale-post").count(),
        1,
        "{kinds:?}"
    );
}
